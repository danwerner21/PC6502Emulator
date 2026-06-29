# DOS/65 Disk and Console Expectations for the PC6502 Emulator

**Investigation date:** 2026-06-27  
**Scope:** PC6502-specific DOS/65, ROM, and banked-driver sources in this repository snapshot  
**Purpose:** Define the software-visible contracts an emulator must satisfy to boot and operate the supplied PC6502 DOS/65 port.

## 1. Confidence model

This specification keeps three evidence classes separate:

- **Confirmed** — directly encoded in the PC6502 build, generated listing, or firmware source.
- **Inferred** — the interpretation that consistently explains the confirmed code, but is not stated by hardware documentation or demonstrated by a supplied runnable disk image.
- **Unknown** — not established by this repository snapshot and therefore not safe to hard-code as historical hardware behavior.

The repository does not contain a ready-to-attach PC6502 disk image or the generated `pcdos65.s19` payload. It contains the source and build recipe needed to determine the expected layout [E5]. The contracts below consequently distinguish byte/address behavior proven by code from image-production details that remain unverified.

## 2. Emulator-facing contract summary

The minimum path to a DOS/65 prompt is:

1. Expose writable low RAM, the selected 4 KiB ROM at `$F000-$FFFF`, the MMU registers, and a 6551-compatible UART before reset.
2. For unattended boot, select the VIDEO ROM bank. It initializes the MMU and UART, then invokes the XT-IDE boot path. The base ROM instead enters Supermon and requires the `B` command [E1, E2].
3. Attach a headerless sector device as XT-IDE master. ATA LBA 0 must be the first 512 bytes intended for CPU address `$0800`.
4. Complete ATA reads for LBAs 0 through 59. The ROM copies the 30,720 bytes to `$0800-$7FFF` and jumps to `$0800` [E12].
5. Preserve the ROM-created MMU maps. The loader copies `$1000-$37FF` to task-0 `$B800-$DFFF`, copies `$4000-$5FFF` to task-1 `$C000-$DFFF`, restores task 0, and jumps to `$B800` [E4].
6. Keep task 0 active for DOS/65. Calls through `$FFF0` must temporarily activate task 1 so the banked dispatcher is visible at `$C000`, then restore task 0 [E9, E10].
7. Implement the selected console's polled input/output behavior. The normal base-ROM setting is serial (`CONSOLE=$04`); the VIDEO setting is mapped video plus keyboard (`CONSOLE=$13`) [E1, E2, E10].
8. Present DOS/65 storage as 512-byte physical sectors while preserving its 128-byte logical-sector view. Four logical sectors share one physical sector, so writes are read-modify-write operations [E7].

An emulator does not need to recognize DOS/65 APIs specially. It must emulate the hardware closely enough that the ROM, SIM, and banked drivers execute these contracts.

## 3. Confirmed boot flow

### 3.1 Reset to ROM boot

Both packaged ROM banks reset at CPU `$F000`. Common startup disables IRQs and decimal mode, initializes the stack, sets RAM IRQ/NMI vectors, initializes MMU task 0 and task 1, initializes the UART, prints a banner, and clears `$0300` [E1, E2].

The two banks then diverge:

| ROM bank | Console selector | Post-initialization behavior |
|---|---:|---|
| Base/serial | `$04` | Executes `BRK`, entering Supermon. Supermon command `B` invokes the ROM IDE boot routine. |
| VIDEO | `$13` (decimal 19) | Jumps directly to the ROM IDE boot routine. |

The build packages base plus VIDEO in `rom.hex`; the separately built ESP variant is not in the supplied ROM image [E5]. Automatic DOS/65 boot therefore requires the VIDEO bank or an explicit Supermon `B` command when using the base bank.

### 3.2 ROM XT-IDE load

The ROM boot routine performs the following fixed sequence [E12]:

1. Initialize/probe XT-IDE.
2. Select ATA master (`currentDrive=0`) and LBA 0.
3. Read one 512-byte sector into `$0400-$05FF`.
4. Copy it to the destination beginning at `$0800`.
5. Increment the physical LBA and destination by 512 bytes.
6. Repeat until the destination reaches `$8000`.
7. Jump to `$0800`.

The exact load is 60 sectors, LBAs `0..59`, into `$0800-$7FFF`. A nonzero return from any sector read branches to the monitor error path and prevents the jump to the partially loaded payload [E12].

The ROM's Supermon `W` command performs the inverse operation after confirmation: it writes `$0800-$7FFF` to XT-IDE LBAs `0..59`. Supermon `L` accepts Motorola S1 records, so the source-intended provisioning flow is **inferred** to be: load the generated `pcdos65.s19` over the console, use `W` to materialize it on disk, then boot [E12, E18].

### 3.3 Staged payload layout

The build constructs `pcdos65.s19` as a sparse address payload [E5]:

| Staging range | Content | Destination after `$0800` loader runs |
|---|---|---|
| `$0800-$085E` | PC6502 loader (95 bytes in the checked-in map) | Executes in place |
| `$1000-$37FF` | 10 KiB DOS staging window; linked OS content begins at `$1000` after relocation | Task-0 `$B800-$DFFF` |
| `$4000-$5FFF` | 8 KiB driver staging window; linked driver content begins at `$4000` after relocation | Task-1 `$C000-$DFFF` |

The checked-in PC6502 OS build occupies `$B800-$D870` (8,305 bytes), and the banked driver build occupies `$C000-$D5B0` (5,553 bytes) [E6, E19]. The loader nevertheless copies the full windows shown above [E4]. A disk-image builder must define bytes in the sparse gaps deterministically; zero-fill is a reasonable emulator-tooling default, but the historical fill byte is **unknown**.

### 3.4 DOS/65 cold initialization

The loader jumps to `$B800`. In the compiled PC6502 OS, `$B800` jumps to SIM cold boot at `$CD2E` [E6]. Cold boot:

1. Uses the `CONSOLE` byte already set by ROM; it does not choose a new console.
2. Copies the 16-byte default drive configuration into writable RAM.
3. Calls banked initialization functions for mapped video, ESP, DSKY, RTC, XT-IDE, CH375, floppy, and Multi-I/O.
4. Ignores the return values from all those initialization calls.
5. Performs warm-boot setup: stack, command-line/DCB pointers, page-one vectors, default DMA buffer, default drive A, and home track.
6. Enters the console command module and emits the DOS prompt [E7].

Absent optional peripherals are therefore not intrinsically fatal. The configured console and the storage device backing drive A are fatal in practice because the command loop and drive login need them.

## 4. Confirmed disk model

### 4.1 DOS logical geometry

All eight PC6502 DCBs use the same geometry [E7]:

| DCB field | Value | Consequence |
|---|---:|---|
| Maximum allocation block | 2047 inclusive | 2,048 allocation blocks |
| Logical sectors per track | 64 | 8 KiB per logical track |
| Reserved system tracks | 16 | 1,024 logical sectors / 128 KiB before the data area |
| Block-size code | 2 | 4,096-byte allocation blocks, per source comment and PEM decoding |
| Maximum directory entry | 511 inclusive | 512 directory entries, 32 bytes each |
| Checksum flag | 128 | Directory checksums disabled for these DCBs |

DOS/65 logical sectors are 128 bytes. Logical sector and track numbers are zero-based in this SIM: the PEM produces a sector remainder in the range `0..63`, and `xlate` returns it unchanged [E7, E14].

The capacity represented by one PC6502 slice is:

```text
system area = 16 tracks * 64 sectors/track * 128 bytes = 0x020000 bytes
data area   = 2048 blocks * 4096 bytes                 = 0x800000 bytes
slice size  =                                               0x820000 bytes
            = 8,519,680 bytes
            = 16,640 physical 512-byte sectors
            = 0x4100 physical sectors
```

The data/directory allocation area begins at physical sector `0x0100` within each slice. A complete slice spans 1,040 logical tracks (`0..1039`).

### 4.2 Logical-to-physical mapping

For XT-IDE and CH375 storage, the drivers convert a DOS track/sector request to a 512-byte physical LBA [E7, E12, E13]:

```text
physical_lba = (slice * 0x4100) + (logical_track * 16) + floor(logical_sector / 4)
subsector    = logical_sector & 3
byte_offset  = subsector * 128
```

The apparently unusual slice calculation in the assembly is intentional: `0x4100` sectors is exactly 8 MiB of allocation blocks plus 128 KiB of system tracks. It is not an 8 MiB (`0x4000`-sector) partition stride.

Reads fetch the complete 512-byte host sector into `$0400-$05FF`, then copy the selected 128-byte quarter to the current DOS DMA address. Writes first read the complete host sector, replace one 128-byte quarter, and write all 512 bytes back [E7]. The emulator must preserve the other three quarters.

There is no interleave or skew in the PC6502 SIM: `xlate` is a no-op [E7].

### 4.3 Default drive assignments

The first byte of each drive configuration entry encodes device type in the high nibble and device unit in the low nibble. The second byte is the slice number [E7].

| DOS drive | Configuration | Backing device | Unit | Slice | Physical LBA range on that device |
|---|---:|---|---:|---:|---:|
| A | `$30,$00` | XT-IDE | 0 (master) | 0 | `$0000-$40FF` |
| B | `$30,$01` | XT-IDE | 0 (master) | 1 | `$4100-$81FF` |
| C | `$10,$00` | CH375/USB storage | 0 | 0 | `$0000-$40FF` |
| D | `$10,$01` | CH375/USB storage | 0 | 1 | `$4100-$81FF` |
| E-H | `$90,$00` | Invalid/unimplemented | — | — | — |

A raw XT-IDE image containing both default A and B slices must therefore contain at least `0x8200` physical sectors, or `0x1040000` bytes (17,039,360 bytes / 16.25 MiB). A single bootable A slice needs `0x4100` sectors. The ROM always boots physical XT-IDE LBA 0 and has no slice selector, so the automatic boot payload belongs at the beginning of slice A.

The first 60 sectors used by the ROM boot payload are within A's 256-sector reserved system area. Ordinary DOS data allocation begins at LBA `0x0100`, so the confirmed boot load does not overlap the allocation area.

### 4.4 Physical image convention

The code sends the computed LBA directly to ATA or CH375 and contains no filesystem header adjustment [E12, E13]. A headerless, sector-zero-first image is therefore the **inferred** emulator image format. No generated image is present to confirm:

- a canonical filename or extension;
- a prescribed total image size beyond the DCB-derived minima;
- the fill byte for unused sectors or sparse payload gaps;
- whether deployments normally pre-create one, two, or more slices;
- a host-side formatter or image manifest.

The statement in `documentation/DOS65_Description.md` that this port uses “ROMWBW track/sector mapping” is consistent with the mapping code but does not provide additional PC6502 image metadata [E15].

## 5. BIOS, SIM, and driver entry points

### 5.1 Fixed ROM services

| CPU address | Contract | Registers / effect |
|---:|---|---|
| `$FFF0` | Banked far call | Reads function number from `farfunct` at zero-page `$32`; switches to MMU task 1; calls dispatcher `$C000`; restores task 0. Function-specific A/X/Y contracts apply. |
| `$FFF3` | Motorola S-record loader | Polled console input; accepts S1 data until S9 terminator and writes record data to its encoded CPU addresses. |
| `$FFF6` | MMU page setup | `A=task`, `X=logical page`, `Y=physical page`; temporarily disables MMU, updates the task entry, repairs the same task-0 entry, and re-enables MMU. |

These stub addresses are fixed across the supplied ROM variants even though their internal targets differ [E1, E9].

### 5.2 Compiled PC6502 SIM table

The PC6502 generated listing places the DOS/65 SIM jump table at `$CBDC` in task 0 [E6]. These absolute addresses apply to this checked-in build; software should prefer the table base plus offsets when rebuilt.

| Address | Offset | Operation | Input | Return |
|---:|---:|---|---|---|
| `$CBDC` | `+0` | Cold boot | — | Does not normally return |
| `$CBDF` | `+3` | Warm boot | — | Does not normally return |
| `$CBE2` | `+6` | Console status | — | `A=$00` none, `A=$FF` available for supplied consoles |
| `$CBE5` | `+9` | Console input | — | Blocking character in A |
| `$CBE8` | `+12` | Console output | A=character | Returns after accepted by device |
| `$CBEB` | `+15` | Printer output | A=character | Device-specific |
| `$CBF4` | `+24` | Home | — | Sets track 0 |
| `$CBF7` | `+27` | Select disk | A=drive, masked to `0..7` | A/Y=DCB pointer |
| `$CBFA` | `+30` | Select track | A=low, Y=high | Saves selection |
| `$CBFD` | `+33` | Select sector | A=low, Y=high | Saves selection |
| `$CC00` | `+36` | Set DMA | A=low, Y=high | Saves 128-byte buffer address |
| `$CC03` | `+39` | Read logical sector | Prior disk/track/sector/DMA selections | `A=$00` success; nonzero failure |
| `$CC06` | `+42` | Write logical sector | Same plus write type already managed by PEM | `A=$00` success; nonzero failure |
| `$CC09` | `+45` | Printer status | — | `A=$01` (always ready) |
| `$CC0C` | `+48` | Read clock | — | Stub: `X=$80`; other returned time bytes are not established |
| `$CC0F` | `+51` | Sector translate | A/Y=logical sector | No-op; A/Y unchanged |

Page-one `$0100` is populated with a warm-boot jump and `$0103` with a PEM jump during SIM warm setup [E7, E8].

### 5.3 Banked dispatcher functions relevant to boot and I/O

The dispatcher is at task-1 `$C000`; `farfunct` at `$32` selects a word in its table [E10].

| Function | Operation | Principal contract |
|---:|---|---|
| 0 | Default console write | A=byte |
| 1 | Default console nonblocking read | A=byte or `$00` |
| 2 | Default console blocking read | A=byte |
| 3 | Default console status | A=`$00` or `$FF` |
| 4-8 | 6551 serial write/read/blocking read/status/init | See section 7 |
| 9-13 | ESP video/keyboard/status/init | Optional ESP hardware |
| 14-18 | ESP video plus Multi-I/O keyboard group | Function 18 is a no-op |
| 19-23 | Mapped video plus Multi-I/O keyboard/status/init | VIDEO ROM's default group |
| 34 | Parallel printer output | A=byte |
| 36 | Multi-I/O initialization | Initializes keyboard/printer hardware |
| 60 | XT-IDE initialization | Return value is ignored by SIM cold boot |
| 61 | XT-IDE read 512-byte sector | Uses disk globals; fills `$0400-$05FF`; A=`$00/$FF` |
| 62 | XT-IDE write 512-byte sector | Uses disk globals and `$0400-$05FF`; A=`$00/$FF` |
| 63 | CH375 initialization | A=`$00/$FF`; ignored by SIM cold boot |
| 64 | CH375 read 512-byte sector | A=`$00/$FF` on normal completion paths |
| 65 | CH375 write 512-byte sector | A=`$00/$FF` on normal completion paths |
| 66-68 | Floppy init/read/write | No-op in this PC6502 driver build |

Functions 0-3 are aliases that add the `CONSOLE` selector to the requested operation and redispatch. Valid selectors must point at a four-function output/read/blocking/status group [E10].

## 6. Status and error behavior

### 6.1 Normal storage status

The interoperable status contract is accumulator-based:

- `A=$00` means successful disk operation.
- Any nonzero value means failure to the PEM; supplied device drivers normally use `$FF`.
- Consumers compare A explicitly, so carry and other CPU flags are not the stable cross-layer status interface [E7, E12-E14].

The IDE driver returns `$FF` when BUSY does not clear, DRQ does not assert, or ATA status bit 0 reports error while waiting for DRQ. It returns `$00` after a complete 512-byte transfer [E12]. The ROM boot path aborts immediately on a nonzero read result.

At the DOS SIM layer, an unsupported device type returns `$FF`. A logical read preserves the physical driver's accumulator result while deblocking the host buffer. A logical write returns the final physical write result [E7].

### 6.2 Read-modify-write limitation

The SIM does not test the preliminary 512-byte read result before modifying one 128-byte quarter and issuing the physical write [E7]. Therefore:

- a failed pre-read can cause stale `$0400-$05FF` bytes to be written into the other three quarters;
- the final status may report success even though unrelated logical sectors were corrupted;
- emulator fault injection should expose this behavior rather than silently repairing it;
- normal emulation must make same-sector reads deterministic and preserve the buffer until the following write.

### 6.3 DOS/65 user-visible handling

The PEM treats zero as success. On nonzero read/write status it prints `PEM ERROR ON <drive> - BAD SECTOR`, then waits for input. Return/Enter ignores the error and continues; any other key triggers a warm boot [E14]. Higher-level CCM commands also have `READ ERROR` and `WRITE ERROR` messages [E20].

Initialization failures are less strict: SIM cold boot ignores returned status from video, ESP, RTC, XT-IDE, CH375, floppy, and Multi-I/O initialization [E7]. Device absence becomes visible later when an operation needs it.

### 6.4 Timeout behavior and source defects

- IDE BUSY and DRQ waits use finite 16-bit polling counters, not wall-clock timers. An emulator need not reproduce physical elapsed time, but must allow status to progress before the counters wrap [E12].
- Serial transmit and blocking receive have no timeout. If TX-ready never asserts or no nonzero input arrives, the guest intentionally remains in its polling loop [E11].
- CH375 polling has a finite nested loop, but its timeout return path restores registers in the wrong order and does not guarantee A=`$FF`. This is a **confirmed source defect**; normal emulation should avoid relying on a particular timeout byte, while tests may document the emitted-code behavior [E13].

## 7. Console and UART contracts

### 7.1 Console selection

`CONSOLE` is zero-page byte `$3A` [E8]. It is a dispatcher base, not an abstract enum:

| Selector | Output/read/blocking/status functions | Hardware dependencies |
|---:|---|---|
| `$04` | 4/5/6/7 | On-board 6551-compatible UART |
| `$09` | 9/10/11/12 | ESP interface |
| `$0E` | 14/15/16/17 | ESP video output plus Multi-I/O keyboard input |
| `$13` | 19/20/21/22 | Mapped video card plus Multi-I/O keyboard |

The base and ESP ROMs print the DOS/65 opening banner. The VIDEO setting (`$13`) suppresses that particular SIM banner, although the CCM still uses its console output functions [E7]. DOS/65 never initializes `CONSOLE` itself, so a direct jump into the OS without prior ROM initialization has undefined console selection.

### 7.2 6551 register and initialization contract

The serial driver accesses four addresses [E11]:

| Address | Register | Boot/driver use |
|---:|---|---|
| `$EF84` | Data | Read received byte; write transmitted byte |
| `$EF85` | Status / software reset on write | Write `$00` during initialization; poll RX-ready bit 3 and TX-ready bit 4 |
| `$EF86` | Command | Write `$0B` |
| `$EF87` | Control | Write `$1E` (internal clock, 9600 baud, 8 data bits, no parity, one stop bit) |

The board-level allocation is `$EF80-$EF8F`, but only `$EF84-$EF87` are exercised [E16]. Mirroring across the rest of the 16-byte allocation is **unknown** and should not be required for DOS/65.

The UART contract required by this software is polled:

- **Transmit:** status bit 4 must eventually become 1. The driver then writes A to `$EF84` and returns. There is no software FIFO and no timeout.
- **Nonblocking receive:** if status bit 3 is 0, return A=`$00`; otherwise read and return `$EF84`.
- **Blocking receive:** repeat nonblocking receive until A is nonzero, then clear bit 7 (`A &= $7F`) and return.
- **Status:** return `$FF` when status bit 3 is 1, otherwise `$00`.
- **Interrupts:** the configured command byte disables the receive interrupt and does not request a transmit interrupt. DOS/65 console operation does not require UART IRQ delivery.

NUL (`$00`) cannot be delivered through the blocking serial API because it is also the no-character sentinel. Bytes `$80-$FF` lose their top bit on the blocking path. These are software semantics the emulator should not “fix.”

The physical board documents a CTS-force-high jumper, but the driver does not inspect a CTS bit directly [E16]. A practical emulator should either model flow control consistently with 6551 TX readiness or default virtual CTS asserted so console output cannot deadlock.

### 7.3 DOS terminal expectations

The compiled console definition advertises 80 columns and 24 lines. Its control bytes include backspace `$08`, forward/form-feed `$0C`, home `$1E`, clear-to-EOL `$01`, and clear-to-end-of-screen `$02` [E6, E7]. Serial output is byte-oriented; the UART layer does not interpret those controls or translate line endings. The attached terminal frontend is responsible for display behavior.

DOS line input recognizes Return `$0D`, backspace `$08`, Ctrl-C `$03`, Ctrl-I/tab `$09`, Ctrl-P `$10`, Ctrl-R `$12`, Ctrl-S `$13`, and Ctrl-X `$18` in the PEM [E8, E14].

## 8. Memory and MMU assumptions

### 8.1 Required mappings

ROM initialization creates [E9]:

- task 0: identity mapping for all sixteen logical 4 KiB pages;
- task 1: identity mapping except logical page `$C` maps to physical page `$10` and logical page `$D` maps to physical page `$11`;
- active task 0 with the MMU enabled.

The resulting DOS layout is:

| Logical range | Task 0 | Task 1 during `$FFF0` far call |
|---|---|---|
| `$0000-$BFFF` | Identity-mapped shared working RAM | Identity-mapped shared working RAM, subject to optional video remapping of page B |
| `$B800-$D870` | DOS/65 OS | Not visible in C/D; task-1 driver occupies those logical pages |
| `$C000-$D5B0` | Overlaps DOS/65 in task 0 | Banked driver at physical pages `$10-$11` |
| `$E000-$EFFF` | I/O decode | I/O decode |
| `$F000-$FFFF` | ROM | ROM |

The shared identity-mapped low memory is essential: `farfunct`/pointers in zero page, the CPU stack, disk globals around `$0600`, and host buffer `$0400-$05FF` must retain the same contents across task switches [E7-E10].

### 8.2 Driver-call sequencing

`JSR $FFF0` pushes a task-0 return address on the shared stack. The ROM switches to task 1, calls `$C000`, receives the driver's `RTS`, restores task 0, and then returns to DOS [E9]. An emulator must apply a task-register write to subsequent instruction and data accesses without corrupting stack or fixed I/O/ROM visibility.

The selected ROM and I/O must remain visible through the observed task switches. The complete priority between arbitrary MMU mappings and fixed ROM/I/O decode remains **unknown**; only the mappings exercised above are required by this DOS/65 path.

### 8.3 Required writable memory

At minimum, boot and DOS operation require writable backing for:

- task-0 identity RAM below `$E000`, including zero page, stack, `$0300`, `$0400-$05FF`, `$0600` work area, `$0800-$7FFF`, and `$B800-$DFFF`;
- physical pages `$10-$11` for task-1 driver storage;
- any optional mapped-video physical page used by the selected VIDEO console.

The emulator must not discard the loader's writes to task-1 `$C000-$DFFF`, because all later console and disk far calls execute from that bank.

## 9. ATA behavior needed by the confirmed path

The XT-IDE driver uses byte-wide registers at even offsets plus a separate high data byte [E12]:

| Address | ATA role |
|---:|---|
| `$E300` | Data low |
| `$E301` | Data high |
| `$E302` | Error / feature |
| `$E304` | Sector count |
| `$E306` | LBA low |
| `$E308` | LBA mid |
| `$E30A` | LBA high |
| `$E30C` | Device/head |
| `$E30E` | Command / status |

Required commands are SET FEATURES `$EF` with feature `$01`, IDENTIFY `$EC` during DOS initialization, READ SECTOR `$20`, and WRITE SECTOR `$30`. Device values `$E0/$F0` select master/slave in LBA mode. Sector count is always 1 for DOS and ROM operations [E12].

Status behavior used by the driver is limited to BUSY bit 7, DRQ bit 3, and ERROR bit 0. A transfer consists of 256 low/high byte pairs, exactly 512 bytes. The probe also writes `$FF` and then `$00` across offsets `$00-$30` from `$E300`; unmapped writes in that probe span must be harmless [E12].

An emulator may implement more complete ATA behavior, but the above is the tested compatibility boundary. IDENTIFY data are printed during DOS initialization but are not used to derive the DCB geometry.

## 10. Inferred expectations

These are recommended implementation choices, not confirmed hardware facts:

1. Treat disk-image byte 0 as XT-IDE LBA 0 with no container header.
2. Zero-fill sparse gaps when materializing `pcdos65.s19` into the `$0800-$7FFF` boot payload.
3. Default to ATA master present with virtual CTS asserted when aiming for first-boot usability.
4. Use at least a `0x4100`-sector image for one DOS drive and `0x8200` sectors for default A/B.
5. Keep optional CH375, mapped video, ESP, printer, and floppy devices configurable; their initialization calls do not prove that all were fitted simultaneously.
6. Treat `CONSOLE=$04` plus the base ROM as the smallest interactive configuration. The VIDEO bank is unattended only if both mapped video and Multi-I/O keyboard dependencies are implemented.
7. Preserve raw physical sectors outside a selected 128-byte quarter exactly during DOS writes.

## 11. Unknowns and implementation cautions

- No ready PC6502 disk image or generated `pcdos65.s19` is present, so exact production image bytes and fill policy are unknown.
- The source does not define a host image filename, removable-media policy, write-protection interface, flush semantics, or behavior past the DCB capacity.
- ATA model/firmware quirks, power-up timing, and whether a real board required additional wait states are unknown.
- The board's UART reference clock, exact 6551 variant, register mirroring, and electrical flow-control behavior are unknown.
- MMU versus fixed I/O/ROM priority for unexercised mappings is unknown.
- The default VIDEO ROM still initializes the serial UART, but the reason and whether production systems also attached a serial terminal are unknown [E2].
- SIM's `rdtime` is a stub for this build; RTC hardware initialization does not make DOS/65 time services complete [E7].
- E-H have nonzero DCB pointers despite default `$90` device entries. Selecting them reaches the DCB, but the first I/O returns invalid-device status; do not treat their DCB presence as usable storage [E7].
- The old `WRITEOS` utility describes a different memory/work-area layout and boot-sector flow. It is not evidence for the PC6502 ROM's fixed 60-sector `$0800-$7FFF` load; use the PC6502 ROM and `pcdos65.s19` build recipe as authoritative [E21].

## 12. Verification checklist

An emulator implementation should demonstrate:

1. Reset reaches `$F000`, initializes the MMU/UART, and the base ROM reaches Supermon without device IRQs.
2. VIDEO boot issues sequential ATA reads for LBA `0..59`, fills `$0800-$7FFF`, and transfers control to `$0800` only after all reads succeed.
3. The loader produces distinct task-0 OS and task-1 driver bytes at overlapping logical `$C000-$DFFF` addresses.
4. A call through `$FFF0` executes dispatcher `$C000` in task 1 and returns to the task-0 caller with A preserved as the function result.
5. UART status bit 4 permits output, status bit 3 controls input availability, and no UART IRQ is needed for a DOS prompt.
6. DOS logical `(track=0, sector=0..3, slice=0)` accesses physical LBA 0 at offsets 0, 128, 256, and 384.
7. `(track=0, sector=4)` accesses LBA 1; `(track=1, sector=0)` accesses LBA 16; slice 1 adds `0x4100`.
8. A 128-byte logical write changes only its selected quarter of the 512-byte backing sector.
9. A failed ROM disk read returns to the monitor error path; a failed DOS read returns nonzero and reaches the PEM bad-sector prompt.
10. Default A and B access nonoverlapping slices of the same XT-IDE master; C and D use equivalent slices on CH375 storage when configured.
11. Unsupported E-H I/O returns failure rather than aliasing a valid device.
12. Optional-device absence does not prevent SIM from continuing through its cold-init call sequence.

## 13. Evidence index

- **[E1]** `specifications/rom-reset.md:7-16,93-175,216-229` — selected ROM banks, fixed vectors, reset divergence, exact 60-sector boot load, and minimum reset contract.
- **[E2]** `PC6502_firmware_source/6502PCbios.asm:21-88,134-177,215-228` — console selection, MMU/UART initialization, monitor versus VIDEO boot, console routes, and fixed ROM service stubs.
- **[E3]** `PC6502_firmware_source/bios_defines.asm:6-15,24-32` — fixed dispatcher/MMU addresses and low-memory work areas.
- **[E4]** `PC6502_firmware_source/loader.asm:8-86` — staging ranges, task switches, full OS/driver copy windows, and jump to `$B800`.
- **[E5]** `PC6502_firmware_source/Makefile:1-15,26-54` — ROM variants and composition of loader, relocated OS, and banked driver into `pcdos65.s19`.
- **[E6]** `DOS65_OS/dos65_os/Makefile:2-23` and `DOS65_OS/dos65_os/dos65_pc6502.lst:204-215,3248-3291` — PC6502 conditional build, `$B800` cold jump, exact SIM addresses, and console definition bytes.
- **[E7]** `DOS65_OS/dos65_os/simrbc.asm:15-48,66-184,193-459,626-725,819-904` — SIM API, cold/warm setup, console forwarding, disk select/read/write/deblocking, DCB geometry, and default drive table.
- **[E8]** `DOS65_OS/dos65_os/dosdefn.asm:8-19,25-100,103-139` — DOS work-memory, zero-page contracts, fixed `$FFF0` far-call entry, control bytes, and page-one definitions.
- **[E9]** `PC6502_firmware_source/bios_pager.asm:8-70,74-104` — task mappings, task switching, page setup, and banked far-call implementation.
- **[E10]** `PC6502_firmware_source/dos65drv.asm:15-123,142-167` and `PC6502_firmware_source/dos65drv.lst:165-185` — dispatcher at `$C000`, complete function table, and default-console redispatch.
- **[E11]** `PC6502_firmware_source/bios_serial.asm:15-98,102-157` — UART addresses, command/control meanings, polling bits, blocking behavior, and `$00/$FF` status.
- **[E12]** `PC6502_firmware_source/bios_ide.asm:11-35,137-202,282-486,508-622,624-750` — XT-IDE register/command/status contract, LBA conversion, 512-byte transfers, ROM boot and boot-image write flows.
- **[E13]** `PC6502_firmware_source/bios_ch375.asm:11-49,188-223,228-362,379-513` — CH375 register protocol, polling/timeout behavior, return conventions, LBA transfer order, and 512-byte chunking.
- **[E14]** `DOS65_OS/dos65_os/pemrbc.asm:13-128,451-631,655-715,1270-1380,1516-1637,1739-1748` — DOS entry semantics, zero-based track/sector production, disk status consumption, bad-sector handling, and console controls.
- **[E15]** `documentation/DOS65_Description.md:1-23` — port identity and ROMWBW mapping compatibility warning.
- **[E16]** `documentation/PC6502_system_documentation.md:17-41` — console connectors/CTS jumper and board-level ACIA/I/O allocations.
- **[E17]** `specifications/hardware-spec.md:39-77,79-99,121-140` — normalized PC6502 address map, MMU baseline, optional device inventory, and DOS/65 integration context.
- **[E18]** `DOS65_OS/supermon/supermon.asm:50-155,1386-1464,1607-1622` — monitor console loop, S1 loader, `L`/`B`/`W` commands, and command dispatch.
- **[E19]** `DOS65_OS/dos65_os/dos65_pc6502.map:1-12`, `PC6502_firmware_source/dos65drv.map:1-24`, and `PC6502_firmware_source/loader.map:1-24` — checked-in linked ranges and sizes.
- **[E20]** `DOS65_OS/dos65_os/ccm215.asm:998-1030` — higher-level CCM read/write error strings.
- **[E21]** `DOS65_OS/dos65_utilities/writeos.asm:1-38,88-172,279-355` — older WRITEOS memory layout and IDE boot-sector mechanism, retained only to identify the incompatibility caution.
