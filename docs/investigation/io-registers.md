# PC6502 RTC, UART, and expansion-slot I/O contract

**Investigation date:** 2026-06-27

**Audience:** emulator implementers

**Scope:** CPU-visible RTC, 6551-compatible console UART, and expansion-slot I/O behavior evidenced by this repository

## Confidence labels

- **Confirmed**: stated by the PC6502 board note and exercised by firmware or a generated listing.
- **Source-observed**: required by checked-in firmware, but not independently confirmed against schematics or hardware.
- **Compatible-model reference**: behavior from a primary component manual whose register model matches the source; the exact fitted part is not known.
- **Unknown**: the repository and consulted primary references do not establish the PC6502-specific behavior.

No schematic, PCB source, programmable-logic source, bill of materials, or logic trace is present in the repository. In particular, do not treat the word “ISAish” as proof of IBM ISA electrical or timing compatibility [L1, A1].

## Emulator-facing contract summary

1. Decode expansion I/O at `$E000-$EF7F`, the console ACIA allocation at `$EF80-$EF8F`, and the RTC at `$EF90-$EF9F` [L1].
2. Within the ACIA allocation, implement the four registers actually selected by firmware at `$EF84-$EF87`. Wider mirroring across `$EF80-$EF8F` is unknown [L2].
3. Model the UART as a classic 6551-compatible ACIA. The boot firmware performs a programmed reset, then selects 9600 baud, 8 data bits, no parity, one stop bit, internal receive clock, transmitter enabled, and receive/transmit interrupts disabled [L2].
4. Model the RTC as sixteen low-nibble registers at `$EF90-$EF9F`. The observed register layout and control writes match the Epson RTC-72421/72423 interface exactly, but the physical PC6502 part number remains unconfirmed [L3, D2].
5. Treat all expansion cards as optional. The source observes Dual-ESP at `$E100-$E102`, CH375/376 at `$E260-$E261`, XT-IDE at selected even addresses in `$E300-$E30E`, and Multi-I/O functions at `$E3F0-$E3F2` and `$E3FE-$E3FF` [L7-L10].
6. The firmware uses polling or direct synchronous register access rather than device IRQ handlers. This does not establish that physical interrupt outputs are disconnected; IRQ routing, polarity, priority, sharing, and per-slot jumper choices remain unknown [L1, L4-L10].

## CPU-visible decode

| CPU address | Function | Confidence | Decode notes |
|---|---|---|---|
| `$E000-$EF7F` | Expansion/“ISA” I/O space | Confirmed allocation | Whether any address bits are omitted by baseboard or card decode is unknown. |
| `$EF80-$EF8F` | 6551-compatible ACIA allocation | Confirmed allocation | Only `$EF84-$EF87` are exercised. `$EF80-$EF83` and `$EF88-$EF8F` may be mirrors, open, or reserved. |
| `$EF90-$EF9F` | Battery-backed RTC | Confirmed allocation and full 16-address access model | Firmware directly accesses offsets `$0-$B` and `$D-$F`; `$C` is not exercised. |
| `$EFA0-$EFCF` | Open | Board-documented | Read value and write behavior are unknown. |
| `$EFD0-$EFDF` | MMU task-map edit window | Confirmed, outside this document | See the dedicated memory/MMU investigation. |
| `$EFE0-$EFEF` | MMU control/status | Confirmed, outside this document | `$EFE6` is described as an “ISA TC” status bit, but its source and clearing behavior are unknown [L11]. |
| `$EFF0-$EFFF` | Open | Board-documented | Read value and write behavior are unknown. |

The board note calls `$E000-$EF7F` “ISA IO SPACE” and lists six “ISAish” slots. This is a CPU memory-mapped window, not x86 port I/O: a 6502 `LDA` or `STA` to these addresses performs the access [L1].

## 6551-compatible console UART

### Address map

The board reserves sixteen addresses, but the firmware adds `$84-$87` to `PC6502_IOSPACE = $EF00` [L2].

| Address | Offset within documented ACIA range | Read | Write | Access effects |
|---|---:|---|---|---|
| `$EF84` | `$04` | Receiver Data Register (RDR) | Transmitter Data Register (TDR) | A receive read clears RDR-full and receive error flags on a classic 6551. A transmit write marks TDR non-empty until the byte moves to the shift register [D1]. |
| `$EF85` | `$05` | Status register | Programmed reset; written value is ignored | Firmware writes `$00` here before configuring the device. Classic-compatible reset clears command bits 0-4 and status overrun; it does not replace hardware reset [D1]. |
| `$EF86` | `$06` | Command register | Command register | Controls parity, echo, transmitter/RTS mode, receive IRQ disable, and DTR/transceiver enable. |
| `$EF87` | `$07` | Control register | Control register | Controls stop bits, word length, receiver clock source, and baud selection. |

**Decode uncertainty:** RS0/RS1 conventionally select four consecutive 6551 registers, but the PC6502 places those registers at offsets 4-7 inside a 16-byte allocation. There is no evidence that offsets 0-3, 8-B, or C-F alias them. Implement exact `$EF84-$EF87` decode first and make mirroring configurable.

### Status register at `$EF85` (read)

| Bit | Name | Meaning when set | Clear/change behavior |
|---:|---|---|---|
| 7 | IRQ | An enabled ACIA interrupt condition occurred | Cleared by reading status; a still-active source can assert it again. |
| 6 | DSR | `/DSR` input is high/not ready | Reflects or latches modem-input state depending on silicon; PC6502 wiring is unknown. |
| 5 | DCD | `/DCD` input is high/no carrier | Receiver behavior may depend on `/DCD`; PC6502 wiring is unknown. |
| 4 | TDRE | Transmitter data register is empty | Firmware spins until this is 1 before writing `$EF84`. |
| 3 | RDRF | Receiver data register is full | Set when a byte transfers into RDR; cleared by reading `$EF84`. Firmware uses this as “character available.” |
| 2 | OVRN | Receive overrun occurred | Cleared after reading RDR on classic-compatible parts. |
| 1 | FE | Framing error occurred | Cleared after reading RDR on classic-compatible parts. |
| 0 | PE | Parity error occurred | Cleared after reading RDR on classic-compatible parts. |

The error bits do not themselves generate an interrupt in the reference behavior. Hardware reset clears IRQ, TDRE becomes 1, RDRF/error bits become 0, and DSR/DCD depend on their pins [D1]. The exact fitted 6551 variant is unknown, so silicon-specific errata—especially the W65C51N transmit-ready behavior—must not be silently assumed.

### Command register at `$EF86`

| Bits | Function | Values |
|---:|---|---|
| 7:6 with bit 5 | Parity mode | Bit 5=`0`: no parity, bits 7:6 ignored. Bit 5=`1`: `00` odd, `01` even, `10` mark transmit/no receive check, `11` space transmit/no receive check. |
| 4 | Receiver echo | `0` normal; `1` echo, requiring transmitter-control bits 3:2=`00`. |
| 3:2 | Transmitter/RTS control | `00`: transmitter off, RTS high, TX IRQ off; `01`: transmitter on, RTS low, TX IRQ on; `10`: transmitter on, RTS low, TX IRQ off; `11`: transmit break, RTS low, TX IRQ off. |
| 1 | Receiver IRQ disable | `0` allows the RDRF receive interrupt when bit 0 enables the device; `1` disables it. |
| 0 | DTR/device enable | `0` disables receiver and interrupts, DTR high; `1` enables the device, DTR low. |

Hardware reset clears the command register. A classic-compatible programmed reset clears bits 4:0 while leaving parity bits 7:5 unchanged [D1]. The source comment says no command bit is affected by software reset, which conflicts with the primary 6551-compatible manual. This conflict has no boot-path effect because firmware immediately writes `$0B` after its reset write [L2].

### Control register at `$EF87`

| Bits | Function | Values |
|---:|---|---|
| 7 | Stop selection | `0`: one stop bit. `1`: normally two; 1.5 for five data bits/no parity; one for eight data bits with parity. |
| 6:5 | Word length | `00` 8 bits; `01` 7; `10` 6; `11` 5. |
| 4 | Receiver clock | `0` external 16× receive clock; `1` same internal baud generator selection as transmitter. |
| 3:0 | Baud selection | See table below. |

| Bits 3:0 | Rate |
|---:|---:|
| `$0` | external transmit clock divided by 16 |
| `$1` | 50 baud |
| `$2` | 75 baud |
| `$3` | 109.92 baud |
| `$4` | 134.58 baud |
| `$5` | 150 baud |
| `$6` | 300 baud |
| `$7` | 600 baud |
| `$8` | 1200 baud |
| `$9` | 1800 baud |
| `$A` | 2400 baud |
| `$B` | 3600 baud |
| `$C` | 4800 baud |
| `$D` | 7200 baud |
| `$E` | 9600 baud |
| `$F` | 19200 baud |

Hardware reset clears the control register. The local comment instead says a programmed reset clears its low four bits, while classic W65C51S reference behavior leaves the entire control register unchanged. As with the command conflict, boot overwrites the register immediately [L2, D1].

### Firmware initialization and polling contract

`SERIALINIT` performs these writes [L2]:

| Sequence | Write | Result |
|---:|---|---|
| 1 | `$00 -> $EF85` | Programmed reset. |
| 2 | `$0B -> $EF86` | No parity or echo; transmitter enabled with RTS low and TX IRQ disabled; receive IRQ disabled; DTR/device enabled. |
| 3 | `$1E -> $EF87` | 9600 baud, 8 data bits, one stop bit, internal receiver clock. |

Transmit is strictly polled: wait for status bit 4, then write one byte to `$EF84`. Receive is polled on status bit 3, then reads `$EF84`. The nonblocking firmware routine returns `$00` both for “no character” and for an actual NUL; the blocking routine loops past NUL and masks received bytes with `$7F`. Those are firmware API limitations, not reasons for the emulated ACIA to discard bit 7 or NUL bytes [L2].

Although the 6551 supports IRQ, the configured `$0B` command disables receiver and transmitter interrupts. The PC6502 schematic is absent, so the connection of the ACIA IRQ output to the CPU remains unknown.

The board note also lists JP3 as “CTS force high,” P18 as the console serial connector, J11 as the TTL console connector, and J12 as TTL-connector power enable [L1]. On a conventional 6551, `/CTS` low enables transmission and high disables it [D1], so the literal JP3 label cannot be converted into a safe modem-input default without a schematic (an external level shifter may invert the signal). Expose CTS, DCD, and DSR as configurable inputs; a permissive console profile may hold them in their active/ready states, but that is an emulator policy, not a confirmed board reset state.

## Battery-backed RTC

### Identity and model boundary

The local board note proves only “battery backed up RTC” plus a dedicated non-rechargeable battery connector [L1]. The firmware exposes sixteen low-nibble registers. Its sequence and layout match the Epson RTC-72421/72423: twelve BCD digit registers, weekday, and control registers D-F [L3, D2]. That is strong functional compatibility evidence, not proof of the fitted package.

Implement the RTC-72421/72423-visible contract below for software compatibility. Keep the emulated part name and interrupt wiring documented as a compatibility model until schematic or package-marking evidence becomes available.

### Raw register map at `$EF90-$EF9F`

Only bits 3:0 are defined for raw RTC registers. The upper nibble observed by the 6502 is unknown; firmware masks every raw data read to `$0F` [L3]. Return the low nibble accurately and use a stable configurable value for bits 7:4.

| CPU address | Offset | Register | Bits 3:0 / valid values | Local use |
|---|---:|---|---|---|
| `$EF90` | `$0` | Seconds ones (`S1`) | BCD 0-9 | Read/write through logical field 0. |
| `$EF91` | `$1` | Seconds tens (`S10`) | bit 3 unused; BCD 0-5 | Read/write through logical field 0. |
| `$EF92` | `$2` | Minutes ones (`MI1`) | BCD 0-9 | Logical field 1. |
| `$EF93` | `$3` | Minutes tens (`MI10`) | bit 3 unused; BCD 0-5 | Logical field 1. |
| `$EF94` | `$4` | Hours ones (`H1`) | BCD 0-9 | Logical field 2. |
| `$EF95` | `$5` | Hours tens (`H10`) | bit 3 unused; bit 2 PM in 12-hour mode; bits 1:0 tens | Logical field 2. Firmware selects 24-hour mode. |
| `$EF96` | `$6` | Day-of-month ones (`D1`) | BCD 0-9 | Logical field 3. |
| `$EF97` | `$7` | Day-of-month tens (`D10`) | bits 3:2 unused; BCD 0-3 | Logical field 3. |
| `$EF98` | `$8` | Month ones (`MO1`) | BCD 0-9 | Logical field 4. |
| `$EF99` | `$9` | Month tens (`MO10`) | bits 3:1 unused; 0-1 | Logical field 4. |
| `$EF9A` | `$A` | Year ones (`Y1`) | BCD 0-9 | Logical fields 5 and 6 both resolve here due to the driver alias. |
| `$EF9B` | `$B` | Year tens (`Y10`) | BCD 0-9 | Logical fields 5 and 6 both resolve here. |
| `$EF9C` | `$C` | Weekday (`W`) | bit 3 unused; binary 0-6 | Not accessed by PC6502 firmware. Day-name mapping is software-defined. |
| `$EF9D` | `$D` | Control D (`CD`) | bit 3 30-second adjust; bit 2 IRQ flag; bit 1 BUSY; bit 0 HOLD | Firmware writes `$00` after each logical-field write. |
| `$EF9E` | `$E` | Control E (`CE`) | bits 3:2 period; bit 1 interrupt/pulse mode; bit 0 mask | Firmware writes `$00` after each logical-field write. |
| `$EF9F` | `$F` | Control F (`CF`) | bit 3 TEST; bit 2 24/12; bit 1 STOP; bit 0 RESET | Firmware writes `$02` before data, then `$01`, `$05`, `$04`. |

Data registers use BCD. Invalid dates/times have unpredictable behavior in the compatible-model manual; emulator validation may reject them, but must not silently normalize values if exact device behavior is a goal [D2].

### Control D at `$EF9D`

| Bit | Name | Semantics in compatible model |
|---:|---|---|
| 3 | 30-second adjust | Writing 1 rounds seconds: 00-29 to `00` without minute carry, 30-59 to `00` with carry. It self-clears and inhibits data-register access for up to 76.3 µs. |
| 2 | IRQ flag | Set while the fixed-period output is active. Writing 0 clears/cancels it; writing 1 has no effect. |
| 1 | BUSY | Read-only. With HOLD=1, `0` permits data access and `1` indicates an increment cycle; with HOLD=0 it reads 1. |
| 0 | HOLD | Writing 1 freezes at most one pending increment and enables meaningful BUSY reads. Clear within one second after access. |

### Control E at `$EF9E`

| Bits | Name | Semantics in compatible model |
|---:|---|---|
| 3:2 | `t1:t0` | `00` 64 Hz; `01` 1 Hz; `10` once per minute; `11` once per hour. |
| 1 | Interrupt/standard | `0` fixed-width pulse output; `1` latched fixed-period interrupt output. |
| 0 | MASK | `0` enables the selected output mode; `1` leaves the open-drain output open/inactive. |

The physical connection of the compatible model's `STD.P` open-drain output to PC6502 IRQ, NMI, a slot signal, or nothing is unknown. Local firmware writes `$00` to this register and never services an RTC interrupt [L3].

### Control F at `$EF9F`

| Bit | Name | Semantics in compatible model |
|---:|---|---|
| 3 | TEST | Manufacturer test control; keep 0. Behavior at 1 is unspecified. |
| 2 | 24/12 | `1` selects 24-hour mode; `0` selects 12-hour mode. Changing it can corrupt hour and later calendar fields on real hardware. |
| 1 | STOP | `1` stops the internal clock; `0` resumes. |
| 0 | RESET | `1` clears subsecond state while asserted; `0` releases reset. |

### Observed logical driver API and side effects

The DOS/65 banked driver publishes function 50 (`RTC_WRITE`), 51 (`RTC_READ`), and 52 (`RTC_INIT`) [L4]. Function 52 only prints the current time; despite an OS comment calling it `RTC_RESET`, it does not initialize raw RTC registers [L3, L5].

For function 51, X selects a two-digit field and Y returns packed BCD while A returns `$00`:

| X | Intended field | Raw offsets read | Notes |
|---:|---|---|---|
| 0 | seconds | 0, 1 | Supported. |
| 1 | minutes | 2, 3 | Supported. |
| 2 | hours | 4, 5 | Supported. |
| 3 | day of month | 6, 7 | Supported. |
| 4 | month | 8, 9 | Supported. |
| 5 | nominal day/weekday in legacy callers | A, B | Actually reads year; this is a source mismatch. |
| 6 | year | A, B | Explicitly decremented to 5 before address calculation. |
| other | unspecified | `(2*X) & $0F`, then next offset | Read has no range guard; do not rely on this accidental wraparound. |

Function 50 accepts only X=0-6; X=6 aliases to X=5. It returns A=`$00` on success or `$FF` for X>=7. A successful field write performs this exact sequence [L3]:

1. Write `$02` to `$EF9F` (STOP=1, 12-hour selection as encoded).
2. Write Y's low nibble to raw offset `2*X` and its high nibble to the next offset.
3. Write `$00` to `$EF9D` and `$EF9E`.
4. Write `$01`, then `$05`, then `$04` to `$EF9F`, ending with 24-hour mode, clock running, reset released.

This sequence is emulation-sensitive. It resets control state after every field write and temporarily rewrites the 24/12 selection. A high-fidelity device model should expose the raw side effects rather than treating function 50 as an atomic host-time update.

### Confirmed RTC software mismatches

- `DOS65_OS/dos65_utilities/rtc.asm` is derived from a DS1302-oriented utility and still calls logical register 7 for write protect and register 8 for trickle charge. PC6502 `RTC_WRITE` rejects both with `$FF`, so these operations have no hardware effect [L3, L6].
- The utility writes a legacy “day” field at X=5 and year at X=6. Both resolve to raw year offsets `$A-$B`; the later year write overwrites the earlier value. Raw weekday `$C` is never written [L3, L6].
- The RTC driver does not use HOLD/BUSY when reading. A value can tear across an increment boundary. The compatible-model manual recommends HOLD/BUSY or two matching reads [D2]. Preserve register-level timing if software tests depend on this race; a simpler emulator may return a coherent snapshot but should document that deviation.
- The compatible model's registers are undefined at power-on, while a battery-backed board normally retains them. For deterministic tests, expose an explicit policy such as persistent image, host-wall-clock initialization, or fixed test epoch. Do not claim one as the physical default [L1, D2].

## Expansion slots and source-observed cards

### Physical slots and IRQ jumpers

The board note names six connectors and one IRQ-assignment jumper per connector [L1]:

| Slot connector | IRQ-assignment jumper |
|---|---|
| J2 | J1 |
| J4 | J2 |
| J6 | J5 |
| J8 | J7 |
| J10 | J9 |
| J14 | J13 |

No source maps a connector to an address subrange, gives jumper positions, or identifies the destination CPU signal. The following are all **unknown**: connector pinout; address/data width; per-slot select; whether cards decode the shared address bus themselves; IRQ versus NMI selection; active level/edge; sharing; priority; acknowledgement; wait states; clock; reset; DMA; bus mastering; terminal count; and open-bus value.

Therefore, emulator configuration should attach optional devices by address, not pretend that a known connector number selects an address. A future schematic may add a connector/IRQ routing layer without changing the device register models.

### Source-observed card summary

| Optional device | Exercised CPU addresses | Interrupt use | Reset/probe behavior |
|---|---|---|---|
| Dual ESP I/O | `$E100-$E102` | Polled only | Sends 32 zero bytes to each endpoint, then opcode `$FF`; expects `ESP32V1`. |
| CH375/376 USB storage | `$E260-$E261` | Polls command-port bit 7 active-low; CPU IRQ routing unknown | Sends command `$05` reset, then `$06/$AA`; expects inverted `$55`. |
| XT-IDE/XT-CF-Lite | selected registers `$E300-$E30E` | Polled only | Firmware also writes `$FF` then `$00` across `$E300-$E330`; selected ATA task-file registers are then probed. |
| Multi-I/O keyboard/LPT | `$E3F0-$E3F2`, `$E3FE-$E3FF` | Keyboard controller configured with interrupts disabled; LPT polled | Keyboard controller self-test `$AA` must return `$55`; LPT data/control are initialized. |

These addresses are firmware choices, not proven slot-number assignments. Card presence by default is unknown [L4, L7-L10].

### Dual ESP I/O at `$E100-$E102`

| Address | Register | Access | Semantics |
|---|---|---|---|
| `$E100` | ESP0 data | R/W | Write opcode or payload when status bit 1 is clear. Read response byte when status bit 0 is set. |
| `$E101` | ESP1 data | R/W | Write when status bit 4 is clear. Read when status bit 3 is set. |
| `$E102` | Shared status | Read | bit 0 ESP0 response ready; bit 1 ESP0 busy; bit 3 ESP1 response ready; bit 4 ESP1 busy. Bits 2 and 5-7 are unknown. |

Firmware waits for BUSY=0 before either a read or write, then additionally waits for RDY=1 before a read. Timeouts set the driver carry flag. A practical emulation is a byte-command FIFO per endpoint plus these four handshake bits; exact queue depth, read-consume behavior, processing latency, reset state, and IRQ capability are not documented [L7].

### CH375/376 USB storage at `$E260-$E261`

| Address | Read | Write | Source-observed behavior |
|---|---|---|---|
| `$E260` | Data/result byte | Command parameter or payload byte | Command-dependent stream port. |
| `$E261` | Interrupt/status pin state | Command byte | Read bit 7: firmware waits until it becomes 0, then writes `GET_STATUS` (`$22`) and reads the result from `$E260`. Other read bits are unused. |

The source defines commands `$01`, `$05`, `$06`, `$0A`, `$0B`, `$15-$17`, `$22`, `$28`, `$2B`, `$31`, `$39-$3E`, `$4D`, and `$51-$59`; only their command-stream behavior, not additional memory-mapped registers, is relevant to decode [L8]. The driver polls the active-low completion indication instead of handling a CPU interrupt. Completion codes exercised include `$14` success and `$16` no media. Command timing, FIFO depth, exact CH375 versus CH376 revision, electrical IRQ routing, and power-on register state are unknown.

### XT-IDE at `$E300-$E30E`

| Address | Read | Write | Firmware use |
|---|---|---|---|
| `$E300` | Data low byte | Data low byte | First byte of each 16-bit ATA data word. |
| `$E301` | Data high byte | Data high byte | Second byte of each ATA word. |
| `$E302` | Error | Features | Writes feature `$01` before command `$EF` to enable XT-CF-Lite 8-bit mode. |
| `$E304` | Sector count | Sector count | Firmware writes 1. |
| `$E306` | LBA low | LBA low | Low LBA byte. |
| `$E308` | LBA mid | LBA mid | Middle LBA byte. |
| `$E30A` | LBA high | LBA high | High LBA byte. |
| `$E30C` | Device/head | Device/head | `$E0` master or `$F0` slave with LBA mode. |
| `$E30E` | Status | Command | Commands exercised include `$20` read, `$30` write, `$EC` identify, and `$EF` set features. |

Only status bit 7 (`BSY`), bit 3 (`DRQ`), and bit 0 (`ERR`) are inspected. Sector data transfers alternate low/high ports for 256 words, producing one 512-byte sector [L9]. Standard ATA behavior is the intended card protocol, but alternate status, device control, IRQ acknowledge, address mirrors, odd unused addresses, and card-specific reset semantics are not established here.

The probe writes 49 bytes starting at `$E300` first with `$FF`, then `$00`. An emulator should tolerate and ignore writes to unused offsets in `$E300-$E330` during probe rather than treating them as fatal unmapped accesses [L9].

### ISA Multi-I/O at `$E3F0-$E3F2` and `$E3FE-$E3FF`

The source defines a nominal card base of `$E3E0`, but performs no access at the base itself [L10].

| Address | Register | Direction | Bits/semantics |
|---|---|---|---|
| `$E3F0` | LPT data | Write | bits 7:0 are printer data PD7:PD0. Initialized to `$00`. |
| `$E3F1` | LPT status | Read | bit 7 `/BUSY`, bit 6 `/ACK`, bit 5 paper-out, bit 4 selected, bit 3 `/ERROR`; bits 2:0 read as/are described as 0. Firmware tests bit 7 only and treats 1 as ready. |
| `$E3F2` | LPT control | Write | bit 7 STAT1, bit 6 STAT0, bit 5 enable, bit 4 printer-interrupt control, bit 3 select, bit 2 reset, bit 1 line feed, bit 0 strobe. Firmware initializes `$08` then `$0C`, and sends a byte with `$0D` then `$0C`. |
| `$E3FE` | Keyboard data | R/W | Read output-buffer/scancode data; write keyboard-controller data after status bit 1 becomes 0. |
| `$E3FF` | Keyboard status/command | Read status; write command | Read bit 0: output data pending; read bit 1: host input buffer busy. Other status bits are unused/unknown. |

The firmware expects an 8042-compatible controller identified in messages as VT82C42. It issues controller self-test command `$AA` and expects `$55`, writes controller command byte `$20` via command `$60` (“translation disabled, mouse disabled, no interrupts”), then resets/configures the keyboard. Controller-command coverage, remaining status bits, mouse port, IRQ routing, and reset defaults are not fully specified [L10].

The LPT driver is a minimal Centronics-style output path. It waits for status bit 7, writes data, pulses control bit 0, and returns carry set on timeout. Signal polarity beyond labels in the source, interrupt operation, bidirectional mode, and electrical timing are unknown [L10].

### Memory-mapped video note

`bios_video.asm` describes another optional expansion device, but it is not in the `$E000-$EF7F` I/O window. Firmware maps physical page `$F8` into logical `$B000` using the MMU. Its register/memory contract belongs with the video and MMU investigations and is intentionally not duplicated here [L12].

## Reset and interrupt matrix

| Device | Hardware/power-on state established? | Firmware-established state | CPU interrupt contract |
|---|---|---|---|
| 6551-compatible UART | Compatible reference defines reset values; exact fitted variant and modem-pin defaults unknown | Program reset, command `$0B`, control `$1E` | RX and TX IRQ disabled; physical IRQ wire unknown. |
| RTC | Compatible reference says all registers undefined at power-on; battery retention policy unknown | DOS boot reads/displays time; field writes force CD=`0`, CE=`0`, CF final=`4` | Compatible RTC supports periodic open-drain output; PC6502 connection unknown and unused. |
| Dual ESP | Unknown | Zero flush and identity probe | No IRQ behavior in source. |
| CH375/376 | Unknown | Command reset and probe | Device completion is polled at `$E261` bit 7; CPU IRQ connection unknown. |
| XT-IDE | Unknown | Broad write sweep, probe, device select, optional feature command | ATA status is polled; card IRQ connection/acknowledge unknown. |
| Multi-I/O | Unknown | Keyboard self-test/configuration; LPT reset/select sequence | Keyboard interrupts explicitly disabled; LPT interrupt not enabled. Slot IRQ wiring unknown. |

## Required emulator tests

1. **UART decode:** accesses to `$EF84-$EF87` reach the four ACIA registers; unproven aliases are disabled by default.
2. **UART boot writes:** `$00 -> EF85`, `$0B -> EF86`, `$1E -> EF87` produces the documented polling configuration.
3. **UART TX/RX flags:** TX bit 4 transitions consistently with writes; injected RX data sets bit 3; reading data consumes it and clears receive errors.
4. **UART NUL/bit 7:** raw device accepts all eight data bits even though the firmware blocking API filters them.
5. **RTC raw BCD:** offsets 0-B and C expose low-nibble digit/weekday data; high nibble policy is stable and documented.
6. **RTC control:** STOP, RESET, 24/12, HOLD/BUSY, periodic-mode mask, and IRQ flag follow the compatible contract or are explicitly marked simplified.
7. **RTC driver sequence:** a function-50-style raw sequence `$02`, digit writes, CD/CE zero, `$01/$05/$04` produces the same side effects as direct register accesses.
8. **RTC mismatches:** logical fields 5 and 6 both observe raw year; writes to legacy logical fields 7 and 8 return failure at the driver layer.
9. **Optional-card absence:** reads/writes in the expansion window do not crash the emulator; absent-device return value is configurable because open-bus behavior is unknown.
10. **ESP handshake:** per-endpoint BUSY/RDY bits gate data-port access and identity probe can return `ESP32V1`.
11. **CH375 polling:** command-port bit 7 goes active-low on completion; `GET_STATUS` returns scripted success/no-media results through the data port.
12. **XT-IDE sector transfer:** 256 low/high word transfers produce exactly 512 bytes; BSY/DRQ/ERR drive the firmware loops; unused probe writes through `$E330` are tolerated.
13. **Multi-I/O:** keyboard self-test returns `$55`; input/output status bits gate `$E3FE`; LPT `/BUSY` and strobe sequence are observable.
14. **Interrupt isolation:** with firmware defaults, normal UART, RTC, keyboard, and storage activity does not spuriously invoke CPU IRQ.

## Unresolved hardware questions

1. Which exact 6551/65C51 variant and reference crystal are fitted?
2. Are `$EF84-$EF87` mirrored within `$EF80-$EF8F`?
3. Is the RTC an RTC-72421/72423 or another register-compatible part, and what appears on CPU bits 7:4 during reads?
4. Is the RTC periodic output wired anywhere, and if so to IRQ, NMI, or a slot signal?
5. What are the six slot connector pinouts, clocks, selects, wait-state rules, reset, and voltage levels?
6. What does each IRQ-assignment jumper position select? Are lines level-sensitive, shareable, or prioritized?
7. How are card address bases configured, and which mirrors/partial decodes exist on real cards?
8. What drives `$EFE6` “ISA TC,” and what read or event clears it?
9. What value does the CPU read from absent cards and documented open regions?
10. Which optional cards were installed in the reference machine and in which physical slots?

## Evidence index

### Local repository

- **[L1]** `documentation/PC6502_system_documentation.md:1-41` — component inventory, six slots and IRQ jumpers, battery/console jumpers, and CPU-visible I/O allocation.
- **[L2]** `PC6502_firmware_source/bios_serial.asm:15-98,102-157` — UART addresses, command/control bit descriptions, init values, and polling routines.
- **[L3]** `PC6502_firmware_source/bios_rtc.asm:11-94,95-180` — RTC base, raw-nibble translation, exact write control sequence, API return behavior, and displayed fields.
- **[L4]** `PC6502_firmware_source/dos65drv.asm:41-137` — banked function dispatch and inclusion of optional device drivers.
- **[L5]** `DOS65_OS/dos65_os/simrbc.asm:66-120` — DOS/65 cold-start calls for RTC and optional-card initialization.
- **[L6]** `DOS65_OS/dos65_utilities/rtc.asm:46-86,328-514,517-593,855-948` and `DOS65_OS/software/dbasic/rtc.asm:3-128` — legacy DS1302 assumptions and higher-level RTC field usage.
- **[L7]** `PC6502_firmware_source/bios_esp.asm:1-44,56-209,294-562` — Dual-ESP ports, status bits, identity probe, opcodes, and handshake loops.
- **[L8]** `PC6502_firmware_source/bios_ch375.asm:1-46,102-226,228-482` — CH375/376 two-port interface, command list, active-low completion polling, and disk transaction flow.
- **[L9]** `PC6502_firmware_source/bios_ide.asm:1-69,137-202,282-486,596-622` — XT-IDE task-file map, probe/reset writes, status polling, and 512-byte transfers.
- **[L10]** `PC6502_firmware_source/bios_multi.asm:1-51,79-262,264-376,1056-1096` — Multi-I/O addresses, LPT bits, keyboard controller status/probe/configuration, and printer handshake.
- **[L11]** `PC6502_firmware_source/bios_pager.asm:8-25` — MMU allocation and `$EFE6` ISA-terminal-count description.
- **[L12]** `PC6502_firmware_source/bios_video.asm:1-131` — optional video card's physical-page mapping, outside the slot I/O window.
- **[A1]** Recursive repository inventory on 2026-06-27 — no schematic, PCB source, programmable-logic source, bill of materials, or logic capture was present.

### Primary compatible-device references

- **[D1]** Western Design Center, [W65C51S ACIA data sheet](https://wdc65xx.com/wdc/documentation/w65c51s.pdf), pp. 6-14 — classic 6551-compatible register selection, bit meanings, reset state, side effects, clocks, modem signals, and open-drain IRQ. The PC6502's exact UART variant is unknown; this reference is used only where it agrees with the local classic-6551 programming model.
- **[D2]** Seiko Epson, [RTC-72421/72423 application manual](https://download.epsondevice.com/td/pdf/app/RTC-72421_en.pdf), pp. 7-18 — exact 16-register BCD layout, control bits, power-on undefined state, access side effects, periodic output, initialization, and HOLD/BUSY procedure. Register compatibility is strong; physical-part identity is not confirmed.
