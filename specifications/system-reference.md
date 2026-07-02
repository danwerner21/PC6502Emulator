# PC6502 System Reference — Integrated Development Reference

**Synthesized:** 2026-06-27  
**Investigation bead:** mc-4c6  
**Audience:** Emulator implementers  
**Purpose:** Single authoritative reference integrating the five source investigations. Reconciles conflicts, ranks unknowns by implementation risk, and supplies domain checklists for memory/MMU, ROM/reset, peripheral I/O, and DOS/65 services.

## Reconciliation — 2026-07-02

**Bead:** mc-hsy. **Commit range reviewed:** `3b64971` (this document's synthesis) through `0322d3c` (current `main` HEAD at review time), i.e. all 20 commits of the M1–M6 milestone implementation plus the post-merge hardening series. Gate tests re-run at HEAD with `PC6502_ROM_HEX`/`PC6502_DISK_IMG` set: **13/13 non-ignored tests pass**, 1 ignored (`m5_com_load`, see §10.5).

This reconciliation cross-checked every claim below against the current `emulator/src/*.rs` implementation, not just against the M1–M6 requirements. Headline findings (full detail inline at point of use, tagged **[2026-07-02]**):

- **Disk content correction (§7.2, §10.5):** the CP/M directory is at LBA `0x0100` (256), not LBA 60; it holds 29 active files (24 `.COM`). This corrects the disk-emptiness assumption in the original `mc-0sj` artifact report, itself already corrected in `plans/pc6502-emulator-milestones/artifact-verification.md` at commit `f48cfbe` — this document had not yet picked that correction up.
- **Two emulator bugs found in the RTC model** (§10.4): a register-offset shift that swaps weekday into the day/month/year field range, and a STOP-bit check against the wrong bit. Both are in `emulator/src/rtc.rs`; the M6 gate test asserts the buggy offsets as correct, so `cargo test` passing does not catch this.
- **Five config knobs created for P0/P1 unknowns are declared but never consulted** by the emulation logic (`cpu_subtype`, `cpu_hz`, `shadow_addr_low`, `acia_variant`, `io_rom_always`; see §9 status column for each). They round-trip through TOML and are covered by `tests/config_load.rs`, which can give a false impression that the underlying unknown is handled.
- **`$EFE6` and `$EFE7` (MMU) are not implemented at all** — both fall through to the generic open-bus arm rather than their documented specific behavior. This is a stronger claim than "unknown": the emulator has not yet made *any* choice here.
- Sections **§9** (ranked unknowns) and **§10** (checklists) now carry an implementation-status annotation per item, dated 2026-07-02, distinct from the original hardware-evidence confidence labels in §1. A status annotation describes what the *emulator* currently does; it does not change the underlying hardware claim, which remains sourced to firmware/board evidence as before.

## How to use this document

Read this file first. Follow source-document links (`→`) when you need the full evidence chain for a specific decision. Every claim below cites at least one of the five investigations; those investigations in turn cite firmware sources and board documentation.

**Source investigation files:**

| Short tag | File | Topic |
|---|---|---|
| HW | `specifications/hardware-spec.md` | Board baseline, component inventory, conflicts |
| MEM | `specifications/memory-mmu.md` | Physical address space, MMU register model |
| ROM | `specifications/rom-reset.md` | ROM images, bank selection, reset flow |
| DOS | `specifications/dos65-expectations.md` | Boot flow, disk model, console and API contracts |
| IO | `docs/investigation/io-registers.md` | UART, RTC, expansion card register maps |

---

## 1. Confidence model

All five source investigations share these four labels. This document inherits them.

| Label | Meaning |
|---|---|
| **Confirmed** | Stated by board documentation and corroborated by firmware, or directly encoded in generated machine-code listings/maps. |
| **Source-observed** | Present in firmware but not independently confirmed against schematics or hardware measurements. |
| **Inferred** | The only consistent interpretation of multiple confirmed facts; must remain configurable until hardware validates it. |
| **Unknown** | The repository does not contain enough evidence to specify the behavior. |

No schematic, PCB design, bill of materials, logic-analyzer capture, or existing emulator is available. This is a functional specification derived from board notes and executable firmware evidence only. → HW §1

---

## 2. System identity

| Property | Status | Value |
|---|---|---|
| Target names | **Confirmed** | "6502PC" (board document) and "PC6502" (firmware/build) are the same board. → HW §2 |
| CPU family | **Confirmed** | 6502 software target — BIOS and DOS/65 build are 6502 assembly. Exact part (NMOS 6502, 65C02 variant) is **unknown**. → HW §2, ROM §7.3 |
| Board title "6809PC" | Resolved conflict | Stale copy/paste error in the board document heading. Body, vectors, source, and listings are all 6502-specific. Emulate a 6502 target. → HW §9, ROM §8 |
| Main RAM | **Documented** | 512 KiB SRAM. Firmware treats pages `$00-$11` as RAM. Page selectors above `$7F` are valid but not backed by installed RAM. → HW §3, MEM §2.1 |
| ROM window | **Confirmed** | 4 KiB CPU-visible window at `$F000-$FFFF`; two banks in an 8 KiB programming file (`rom.hex`). → HW §3, ROM §1 |
| MMU | **Confirmed functional dependency** | 64 task contexts, 16 physical-page entries per task, programmed by BIOS before enabling. → MEM §4 |
| Console UART | **Confirmed** | 6551-compatible ACIA at `$EF84-$EF87`. → IO UART section |
| RTC | **Confirmed allocation; chip unconfirmed** | Battery-backed; firmware model matches Epson RTC-72421/72423. → IO RTC section |
| Expansion slots | **Confirmed** | Six "ISAish" slots; allocation `$E000-$EF7F`. Electrical details unknown. → HW §3, IO slot section |
| BIOS authorship | **Confirmed** | Dan Werner; original 2014-01-01, cleanup 2023-01-22, PC6502 port 2025-12-06. → HW §8, ROM §3.1 |

---

## 3. Integrated CPU-visible memory map

This section is the merged authoritative decode table. It resolves the `$E000` boundary conflict present in the board document. → HW §4, MEM §3.1, IO §CPU-visible decode

| CPU address range | Device or function | Confidence | Notes |
|---|---|---|---|
| `$0000-$DFFF` | RAM (MMU-translated when enabled) | **Confirmed** | Board document printed `$0000-$E000`; normalizing to `$DFFF` is the only non-overlapping reading. I/O takes priority at `$E000`. → HW §9 conflict 1, MEM §3.1 |
| `$E000-$EF7F` | Expansion/"ISA" I/O | **Documented** | Firmware exercises `$E100-$E102` (ESP), `$E260-$E261` (CH375), `$E300-$E30E` (XT-IDE), `$E3F0-$E3FF` (Multi-I/O). → IO §expansion |
| `$EF80-$EF8F` | 6551-compatible ACIA | **Confirmed allocation** | Firmware uses only `$EF84-$EF87`. Mirroring within the 16-byte range is **unknown**. → IO §UART |
| `$EF90-$EF9F` | Battery-backed RTC | **Confirmed** | All 16 offsets accessed; offsets `$D-$F` are control registers. → IO §RTC |
| `$EFA0-$EFCF` | Reserved / open | **Documented** | Read and write behavior **unknown**. Do not mirror neighboring hardware without evidence. |
| `$EFD0-$EFDF` | MMU task-map edit window | **Confirmed** | 16-entry window for the task selected by `$EFE1`. → MEM §5.1 |
| `$EFE0-$EFEF` | MMU control and status | **Confirmed allocation; partial** | See MMU register table in §4.2. Several offsets unassigned. → MEM §5.2 |
| `$EFF0-$EFFF` | Reserved / open | **Documented** | Read and write behavior **unknown**. → HW §4 |
| `$F000-$FFFF` | Selected 4 KiB ROM bank | **Confirmed** | Visible during reset vector fetch and all task maps that preserve logical page `$F` identity. → ROM §3, MEM §3.1 |

### 3.1 Physical decode precedence

1. I/O and ROM overlays (physical pages `$0E` and `$0F` in the default firmware configuration) take priority over hidden SRAM.
2. MMU translation produces a physical address; the board decode then selects RAM, I/O, ROM, expansion, or a hole.
3. A write to ROM must not update hidden SRAM. An unmapped physical hole silently discards writes and returns a configured open-bus value. → MEM §2.2, §9.3

### 3.2 Physical address space

The MMU produces 20-bit physical addresses: 256 physical pages of 4 KiB each. 512 KiB SRAM occupies physical `$00000-$7FFFF`. Physical pages above `$7F` may be expansion hardware, an I/O/ROM shadow, or an unclaimed hole — **none of them should be backed silently by RAM**. → MEM §2.1, §2.2

---

## 4. MMU specification

### 4.1 Terminology

| Term | Definition |
|---|---|
| Logical page | CPU address bits 15-12 (`$0-$F`), selecting 4 KiB within the 64 KiB CPU space |
| Physical page | 8-bit selector; becomes physical address bits 19-12 |
| Task | One of 64 independent logical-to-physical mapping contexts (`$00-$3F`) |
| Edit selector | The task whose 16-entry window is exposed at `$EFD0-$EFDF` |
| Active task | The task applied to all CPU accesses when the MMU is enabled |

### 4.2 MMU register map

Addresses below assume I/O is visible in logical page `$E`. If another task maps the physical I/O page to logical page `$x`, the same offsets appear at `$xFD0-$xFEF`. → MEM §5

| Address | Access | Bits and effect | Reset / power-on |
|---|---|---|---|
| `$EFD0-$EFDF` | R/W | Edit window: byte at offset `$n` is the physical-page selector for logical page `$n` of the currently selected edit task | SRAM — indeterminate; do not promise any value |
| `$EFE0` | Write only | Active-task selector; bits 5:0 select task `$00-$3F`; bits 7:6 ignored | Hardware reset clears to task 0 |
| `$EFE1` | Write only | Edit-task selector; bits 5:0; bits 7:6 ignored | No documented reset value; software must write before using `$EFD0-$EFDF` |
| `$EFE2` | Write only | MMU enable latch; bit 0: `0` = bypass, `1` = enable; other bits no effect | Hardware reset clears to 0 (disabled) |
| `$EFE3` | Unassigned | No defined register | Read/write behavior **unknown** |
| `$EFE4` | Read only | Bits 5:0 = active task; bit 7 = MMU-enable status (V1.1 schematic); bit 6 unassigned | Per above reset values |
| `$EFE5` | Unassigned | No defined register | Read/write behavior **unknown** |
| `$EFE6` | Read w/ side effect | Reading asserts ISA terminal-count (`TC`) bus signal; returned data not defined | Pulse width and clearing **unknown** |
| `$EFE7` | Read only | Bits 3:0 = current I/O-page value; bits 7:4 not meaningful | Behavior under nondefault shadow straps **unknown** |
| `$EFE8-$EFEF` | Unassigned | No defined registers | Read/write behavior **unknown** |

**Note:** The `$EFD0-$EFEF` block is allocated, not mirrored. No evidence shows any register mirrored outside this block. → MEM §5.2

### 4.3 Task-map store

| Property | Value |
|---|---|
| Task contexts | 64 (`$00-$3F`) |
| Entries per task | 16 (one per logical page `$0-$F`) |
| Entry width | 8 bits (physical page selector) |
| Page size | 4 KiB (CPU bits 11:0 pass through) |
| Total map bytes | 1,024 (64 × 16) |

Map SRAM is volatile; power-on contents are indeterminate. BIOS always writes tasks 0 and 1 before enabling the MMU. Tasks 2-63 remain indeterminate after BIOS init. → MEM §4, §6.1

### 4.4 Reset and BIOS-initialized state

**Hardware reset:**
- MMU enable → 0 (disabled)
- Active task → 0
- Edit-task selector: not established by hardware reset
- Map SRAM contents: indeterminate (emulator may use deterministic bytes internally but must not promise them to software)

**After `INITPAGES` completes:** → MEM §6.2, ROM §6.1, DOS §3.1

| Logical page | CPU range | Task 0 physical | Task 1 physical |
|---|---|---|---|
| `$0` | `$0000-$0FFF` | `$00` | `$00` |
| `$1` | `$1000-$1FFF` | `$01` | `$01` |
| `$2` | `$2000-$2FFF` | `$02` | `$02` |
| `$3` | `$3000-$3FFF` | `$03` | `$03` |
| `$4` | `$4000-$4FFF` | `$04` | `$04` |
| `$5` | `$5000-$5FFF` | `$05` | `$05` |
| `$6` | `$6000-$6FFF` | `$06` | `$06` |
| `$7` | `$7000-$7FFF` | `$07` | `$07` |
| `$8` | `$8000-$8FFF` | `$08` | `$08` |
| `$9` | `$9000-$9FFF` | `$09` | `$09` |
| `$A` | `$A000-$AFFF` | `$0A` | `$0A` |
| `$B` | `$B000-$BFFF` | `$0B` | `$0B` |
| `$C` | `$C000-$CFFF` | `$0C` | `$10` |
| `$D` | `$D000-$DFFF` | `$0D` | `$11` |
| `$E` | `$E000-$EFFF` | `$0E` | `$0E` |
| `$F` | `$F000-$FFFF` | `$0F` | `$0F` |

Final control state after init: active task = 0, edit selector = 0, MMU enabled. The mapping makes task 0 identity-mapped and task 1 identical except logical `$C000-$DFFF` maps to physical `$10000-$11FFF`.

### 4.5 `SETPAGE` service (`$FFF6`)

Calling convention: A = task, X = logical page (must be `$0-$F`), Y = physical page. → MEM §8, DOS §5.1

The routine:
1. Saves A, disables MMU, sets edit task to A.
2. Writes Y to edit-window entry X.
3. Sets edit task to 0 and writes X to task-0 entry X (restoring identity for task 0 at that logical page).
4. Re-enables MMU and returns.

**Critical edge cases:**
- Leaves edit selector at task 0.
- Does not preserve a previously disabled MMU state — always re-enables.
- Does not preserve nonidentity task-0 entries at X.
- A = 0 cannot create a nonidentity task-0 map because step 3 overwrites Y with X.
- X outside `$0-$F` escapes the edit window and can corrupt MMU control registers.
- Does not disable interrupts during its MMU-off interval.

### 4.6 Task-0 alias compatibility quirk

On some hardware revisions, every edit-window write also writes task 0. BIOS initializes task 1 before task 0 to work around this. `SETPAGE` step 3 is the software mitigation. **Default emulator behavior: normal (only the selected edit task is updated).** Implement the alias defect as an optional named compatibility mode. → MEM §8, HW §9 conflict, MEM §13

### 4.7 Task switching and address coherence

- An `$EFE0` write changes the active task latch immediately at the end of that write bus cycle.
- The write itself is decoded using the old mapping; the next cycle uses the new mapping.
- Mapping logical page `$E` away from the physical I/O page removes MMU registers from their normal addresses.
- Mapping logical page `$F` away from the physical boot page removes ROM and hardware vectors.
- Two logical pages mapped to the same physical page are coherent aliases.
- Writes to ROM must not fall through to hidden SRAM. → MEM §3.2, §7.2, §7.3

---

## 5. ROM and reset specification

### 5.1 ROM image structure

`rom.hex` is an Intel HEX 8 KiB programming image spanning `$6000-$7FFF`. It contains two independently selectable 4 KiB banks. Hardware jumper K1 selects which bank is CPU-visible at `$F000-$FFFF`. **K1 default is unknown.** → ROM §1, §3, §4

| Image region | Build | CPU window | Reset target |
|---|---|---|---|
| `$6000-$6FFF` | Base / serial monitor | `$F000-$FFFF` | `$F000` → monitor via `BRK` |
| `$7000-$7FFF` | VIDEO / auto-boot | `$F000-$FFFF` | `$F000` → direct IDE boot |

Address translation: `cpu_address = $F000 + (image_address - selected_bank_start)`.

**Do not map the HEX record addresses `$6000/$7000` directly into the 6502 CPU space.** → ROM §4

### 5.2 Exact hardware vectors

All vectors are little-endian (low byte at lower address). → ROM §5

| Bank | NMI `$FFFA-$FFFB` | RESET `$FFFC-$FFFD` | IRQ `$FFFE-$FFFF` |
|---|---|---|---|
| Base | `59 F0` → `$F059` | `00 F0` → `$F000` | `43 F0` → `$F043` |
| VIDEO | `60 F0` → `$F060` | `00 F0` → `$F000` | `4A F0` → `$F04A` |

### 5.3 ROM service jump table

Fixed stub addresses at `$FFF0-$FFF8` — call these stubs, not their internal targets, as internal targets differ between variants. → ROM §5.3, DOS §5.1

| Stub CPU address | Base internal | VIDEO internal | Service |
|---|---|---|---|
| `$FFF0` | `$FD43` | `$FD4A` | Banked far call (reads `farfunct` at `$32`, switches to task 1, calls dispatcher `$C000`, restores task 0) |
| `$FFF3` | `$F7EE` | `$F7F5` | Motorola S-record loader |
| `$FFF6` | `$FD24` | `$FD2B` | MMU page setup (`SETPAGE`) |

### 5.4 Common reset path

Both banks execute this sequence from `$F000`: → ROM §6.1, DOS §3.1

1. `SEI`, `CLD`, `LDX #$FF`, `TXS` — disable IRQs, clear decimal mode, set stack.
2. Startup delay loop (uncalibrated; real duration depends on unknown CPU clock).
3. Store console selector at zero-page `$3A` (`CONSOLE`): base stores `$04` (serial); VIDEO stores `$13` (mapped video + Multi-I/O keyboard).
4. Install RAM indirect IRQ/NMI vectors at `$0035-$0038`.
5. Call `INITPAGES` — builds task 0 and task 1 maps, selects task 0, enables MMU.
6. Call `SERIALINIT` (both variants — the VIDEO bank still requires a functional UART).
7. Switch to task 1, print startup banner, return to task 0.
8. Clear `$0300`.
9. **Base:** execute `BRK` → enter Supermon monitor.  
   **VIDEO:** jump to `BOOT` → XT-IDE auto-load.

### 5.5 VIDEO bank: automatic boot flow

The ROM reads 60 sectors (LBA 0-59) sequentially from XT-IDE master into staging buffer `$0400-$05FF`, then copies each sector to RAM starting at `$0800`. After sector 59, destination = `$8000`. The ROM then jumps to `$0800`. Any sector-read failure branches to the monitor error path. → ROM §6.3, DOS §3.2

```
RESET $F000
  └─ common init (MMU, UART, banner)
     ├─ base: BRK → Supermon ('B' command invokes BOOT)
     └─ VIDEO: BOOT
           └─ XT-IDE LBA 0..59 → $0800..$7FFF → JMP $0800
                 └─ DOS/65 loader
                       ├─ task 0: copy $1000-$37FF → $B800-$DFFF
                       ├─ task 1: copy $4000-$5FFF → $C000-$DFFF
                       └─ task 0: JMP $B800 (DOS cold boot)
```

### 5.6 ROM-facing emulator requirements

1. Parse `rom.hex` as an 8 KiB image or accept two separate 4 KiB binary banks.
2. Expose a K1-equivalent configuration (bank selector applied at reset, not at runtime by software).
3. Map only the selected bank to `$F000-$FFFF` before the reset vector fetch.
4. Start with MMU disabled, writable low RAM, and I/O/ROM decode active.
5. Preserve ROM visibility through task-0/task-1 maps (both preserve logical page `$F` = `$0F`).
6. Support serial and MMU writes made during the common reset path.
7. Base bank: deliver `BRK` through `$FFFE-$FFFF` so Supermon is reached.
8. VIDEO bank: provide XT-IDE behavior to load 60 sectors, or expose the ROM error path.
9. Treat hot bank-switching, ROM/MMU priority for arbitrary page-F remaps, CPU subtype, timing, and physical K1 default as **unknown / configurable**. → ROM §9, §10

---

## 6. Peripheral I/O

### 6.1 6551-compatible UART (`$EF84-$EF87`)

**Board allocation:** `$EF80-$EF8F`. **Exercised by firmware:** `$EF84-$EF87` only. Mirroring within the 16-byte allocation is **unknown**. Implement exact offsets first; make mirroring configurable. → IO §UART, HW §6.1, DOS §7.2

| Address | R | W | Description |
|---|---|---|---|
| `$EF84` | Receiver Data Register | Transmitter Data Register | RDR read clears RDRF and error flags; TDR write begins transmit |
| `$EF85` | Status register | Programmed reset (written value ignored) | Boot writes `$00` here before configuring |
| `$EF86` | Command register | Command register | Controls parity, echo, TX/RTS mode, RX IRQ, DTR |
| `$EF87` | Control register | Control register | Controls stop bits, word length, receiver clock, baud rate |

**Status register bits at `$EF85`:**

| Bit | Name | Emulator-critical behavior |
|---|---|---|
| 7 | IRQ | Cleared by reading status; active source can reassert |
| 6 | DSR | Modem input — PC6502 wiring **unknown** |
| 5 | DCD | Modem input — PC6502 wiring **unknown** |
| 4 | TDRE | Firmware spins on this bit before writing `$EF84`; must eventually become 1 |
| 3 | RDRF | Set when byte moves into RDR; cleared by reading `$EF84` |
| 2 | OVRN | Overrun — cleared after RDR read on classic-compatible parts |
| 1 | FE | Framing error — cleared after RDR read |
| 0 | PE | Parity error — cleared after RDR read |

**BIOS initialization sequence (must reproduce for correct boot):** → IO §UART init, DOS §7.2

```
$00 → $EF85  (programmed reset)
$0B → $EF86  (no parity, no echo, TX enabled, RTS low, TX IRQ disabled, RX IRQ disabled, DTR low)
$1E → $EF87  (9600 baud, 8 data bits, 1 stop bit, internal clock, bit 4 = receiver internal clock)
```

**Firmware polling contract:**
- **Transmit:** spin on TDRE (bit 4 of `$EF85`); write byte to `$EF84`. No timeout.
- **Nonblocking receive:** check RDRF (bit 3); return `$00` if clear; read `$EF84` if set.
- **Blocking receive:** repeat nonblocking until nonzero, then mask with `$7F`.
- **Status:** return `$FF` when RDRF=1, else `$00`.
- **IRQs:** command `$0B` disables both RX and TX interrupts. DOS/65 does not require UART IRQ delivery.

**Firmware API limitations the emulator must not "fix":** NUL `$00` is indistinguishable from "no character" in the blocking API; bytes `$80-$FF` lose bit 7 on the blocking path. These are software constraints, not emulated ACIA behavior. → DOS §7.2, IO §UART init

**CTS/DCD/DSR:** board has a CTS-force-high jumper. Expose modem inputs as configurable; default virtual CTS asserted to prevent TX deadlock. → IO §UART init

**UART variant caution:** W65C51N silicon has a known transmit-ready errata. Do not silently assume a specific silicon revision. → IO §status register

### 6.2 Battery-backed RTC (`$EF90-$EF9F`)

**Chip identity:** **Unconfirmed.** Firmware model matches the Epson RTC-72421/72423 exactly, but the physical part is unknown. Implement the RTC-72421/72423 register model for software compatibility; document the model as unconfirmed. → IO §RTC identity, HW §6.1

**Read behavior:** Only bits 3:0 are defined. Firmware masks every read with `$0F`. Return accurate low nibble; use a stable configurable value for bits 7:4. → IO §RTC register map

#### Raw register map

| Address | Offset | Register | Valid bits 3:0 | Notes |
|---|---|---|---|---|
| `$EF90` | `$0` | Seconds ones (`S1`) | BCD 0-9 | Field 0 |
| `$EF91` | `$1` | Seconds tens (`S10`) | BCD 0-5; bit 3 unused | Field 0 |
| `$EF92` | `$2` | Minutes ones (`MI1`) | BCD 0-9 | Field 1 |
| `$EF93` | `$3` | Minutes tens (`MI10`) | BCD 0-5; bit 3 unused | Field 1 |
| `$EF94` | `$4` | Hours ones (`H1`) | BCD 0-9 | Field 2 |
| `$EF95` | `$5` | Hours tens (`H10`) | Bit 3 unused; bit 2 PM in 12-hr; bits 1:0 tens | Field 2; firmware selects 24-hr |
| `$EF96` | `$6` | Day ones (`D1`) | BCD 0-9 | Field 3 |
| `$EF97` | `$7` | Day tens (`D10`) | Bits 3:2 unused; BCD 0-3 | Field 3 |
| `$EF98` | `$8` | Month ones (`MO1`) | BCD 0-9 | Field 4 |
| `$EF99` | `$9` | Month tens (`MO10`) | Bits 3:1 unused; 0-1 | Field 4 |
| `$EF9A` | `$A` | Year ones (`Y1`) | BCD 0-9 | Fields 5 & 6 (see driver bug) |
| `$EF9B` | `$B` | Year tens (`Y10`) | BCD 0-9 | Fields 5 & 6 |
| `$EF9C` | `$C` | Weekday (`W`) | Binary 0-6; bit 3 unused | Not accessed by PC6502 firmware |
| `$EF9D` | `$D` | Control D | bit3=30s adjust, bit2=IRQ flag, bit1=BUSY, bit0=HOLD | Firmware writes `$00` after each field write |
| `$EF9E` | `$E` | Control E | bits3:2=period, bit1=irq/pulse mode, bit0=MASK | Firmware writes `$00` after each field write |
| `$EF9F` | `$F` | Control F | bit3=TEST, bit2=24/12, bit1=STOP, bit0=RESET | Firmware writes `$02`, then data, then `$01`/`$05`/`$04` |

**Banked driver field-write sequence (emulation-sensitive):** → IO §RTC driver API, DOS §7 (indirectly)

```
$02 → $EF9F  (STOP=1, 12-hr mode temporarily selected)
 [write Y low nibble → raw offset 2*X, Y high nibble → next offset]
$00 → $EF9D
$00 → $EF9E
$01 → $EF9F  (STOP=0, 24-hr, RESET=1)
$05 → $EF9F  (STOP=0, 24-hr, RESET=1, TEST=1)
$04 → $EF9F  (STOP=0, 24-hr, RESET=0 — clock running)
```

**Known RTC software mismatches (do not "fix" these):** → IO §RTC mismatches

- Driver logical fields 5 and 6 both access raw offsets `$A-$B` (year). Raw weekday `$C` is never written by PC6502 firmware.
- Legacy DS1302 commands (write-protect at X=7, trickle charge at X=8) return `$FF` at the driver layer.
- The driver does not use HOLD/BUSY when reading; values may tear across an increment cycle.
- Register-level content is undefined at power-on. Implement explicit policy: persistent RTC image, host-clock initialization, or fixed test epoch.

### 6.3 Expansion cards (optional)

All expansion cards are **optional**. Firmware absence does not prevent DOS/65 cold boot from continuing past initialization of missing devices. Card presence by default is **unknown**. → IO §expansion, DOS §3.4, HW §6.2

| Optional device | CPU addresses | Required for minimum boot |
|---|---|---|
| XT-IDE (ATA storage) | `$E300-$E30E`, even-spaced | Yes — for VIDEO bank auto-boot and drive A |
| CH375/376 USB storage | `$E260-$E261` | No — drives C/D only |
| Dual ESP I/O card | `$E100-$E102` | No — optional console/network |
| ISA Multi-I/O | `$E3F0-$E3F2`, `$E3FE-$E3FF` | Required only if console selector `$13` (Multi-I/O keyboard + mapped video) |
| Memory-mapped video | Physical pages `$F8-$F9` mapped via MMU into logical `$B000` | Required only for VIDEO console selector `$13` |

#### XT-IDE at `$E300-$E30E` → IO §XT-IDE, DOS §9

| Address | Read | Write |
|---|---|---|
| `$E300` | Data low | Data low |
| `$E301` | Data high | Data high |
| `$E302` | Error | Features |
| `$E304` | Sector count | Sector count |
| `$E306` | LBA low | LBA low |
| `$E308` | LBA mid | LBA mid |
| `$E30A` | LBA high | LBA high |
| `$E30C` | Device/head | Device/head |
| `$E30E` | Status | Command |

Status bits used: BSY (7), DRQ (3), ERR (0). Transfer: 256 low/high byte pairs = 512 bytes. Required commands: SET FEATURES `$EF` (feature `$01`), IDENTIFY `$EC`, READ SECTOR `$20`, WRITE SECTOR `$30`. Device `$E0`/`$F0` = master/slave LBA mode. Sector count always 1. Probe writes `$FF` then `$00` across `$E300-$E330` — tolerate writes to unused offsets without treating them as fatal. → IO §XT-IDE, DOS §9

#### CH375/376 at `$E260-$E261` → IO §CH375

| Address | Read | Write |
|---|---|---|
| `$E260` | Data / result | Command parameter / payload |
| `$E261` | Status (bit 7 active-low completion) | Command byte |

Firmware polls `$E261` bit 7 until it becomes 0, then reads result via `$E260`. Timeout return path has a known register-restore defect; do not rely on a specific timeout return value in A. → IO §CH375, DOS §6.4

#### Dual ESP at `$E100-$E102` → IO §ESP

| Address | Read | Write |
|---|---|---|
| `$E100` | ESP0 response byte when status bit 0 = 1 | ESP0 opcode/payload when status bit 1 = 0 |
| `$E101` | ESP1 response byte when status bit 3 = 1 | ESP1 opcode/payload when status bit 4 = 0 |
| `$E102` | Status: bit 0 ESP0 RDY, bit 1 ESP0 BSY, bit 3 ESP1 RDY, bit 4 ESP1 BSY | — |

#### ISA Multi-I/O at `$E3F0-$E3FF` → IO §Multi-I/O

| Address | Read | Write |
|---|---|---|
| `$E3F0` | — | LPT data |
| `$E3F1` | LPT status (bit 7 `/BUSY`; bit 7=1 means ready) | — |
| `$E3F2` | — | LPT control |
| `$E3FE` | Keyboard data / scancode | Write keyboard controller data |
| `$E3FF` | Status: bit 0 output data pending, bit 1 input buffer busy | Keyboard controller command |

Keyboard self-test: write command `$AA`, expect response `$55`. Controller command byte `$60` disables interrupts, mouse, and scancode translation. → IO §Multi-I/O

---

## 7. DOS/65 services and boot contract

### 7.1 Boot sequence summary → DOS §3, ROM §6

```
1. Hardware reset → ROM $F000 (common init: MMU, UART, banner, $0300=0)
2a. Base ROM: BRK → Supermon, 'B' command → BOOT
2b. VIDEO ROM: direct BOOT
3. BOOT: XT-IDE LBA 0..59 → staging $0400-$05FF → copy to $0800-$7FFF → JMP $0800
4. Loader at $0800:
   - Task 0 active: copy staging $1000-$37FF → logical $B800-$DFFF
   - Task 1 active: copy staging $4000-$5FFF → logical $C000-$DFFF (phys $10000-$11FFF)
   - Restore task 0: JMP $B800
5. DOS/65 cold boot at $B800 → $CD2E (SIM cold init):
   - Uses CONSOLE byte set by ROM; does not reset console selection
   - Calls optional-device init (video/ESP/RTC/XT-IDE/CH375/floppy/Multi-I/O) — ignores return values
   - Warm-boot setup → DOS prompt
```

### 7.2 Disk model → DOS §4

**Logical geometry (all 8 DCBs identical):**

| Field | Value |
|---|---|
| Logical sector size | 128 bytes |
| Logical sectors per track | 64 |
| System tracks reserved | 16 |
| Allocation block size | 4,096 bytes (code 2) |
| Maximum allocation blocks | 2,048 |
| Maximum directory entries | 512 |
| One-slice capacity | 8,519,680 bytes (16,640 physical 512-byte sectors = `$4100` sectors) |

**Logical → physical LBA mapping:**

```
physical_lba = (slice × $4100) + (logical_track × 16) + floor(logical_sector / 4)
byte_offset  = (logical_sector & 3) × 128
```

No interleave or skew (`xlate` is a no-op). Four logical 128-byte sectors share each 512-byte physical sector. Writes must read-modify-write: read full 512 bytes, replace the target 128-byte quarter, write all 512 bytes back. → DOS §4.2

**Default drive assignments:**

| Drive | Device | Unit | Slice | LBA range on device |
|---|---|---|---|---|
| A | XT-IDE master | 0 | 0 | `$0000-$40FF` |
| B | XT-IDE master | 0 | 1 | `$4100-$81FF` |
| C | CH375 | 0 | 0 | `$0000-$40FF` |
| D | CH375 | 0 | 1 | `$4100-$81FF` |
| E-H | Invalid | — | — | Returns failure |

**Minimum XT-IDE image sizes:** one-slice (A only): `$4100` sectors; two-slice (A+B): `$8200` sectors. ROM boot occupies LBAs `0..59` (within the 256-sector system area). The allocation area begins at LBA `$0100`. → DOS §4.3

**[2026-07-02] Confirmed content of the shipped `emulator/disk_image/disk.img`:** LBAs 0–59 are the boot payload (60 sectors, copied by the VIDEO ROM to `$0800-$7FFF`). LBAs 60–255 are the unused tail of the 256-sector reserved system area, `$E5`-filled. The CP/M directory itself starts at LBA `$0100` (256) as the geometry above implies, and — checked directly against the shipped image, not inferred — **is populated**: 29 active entries (481 of the 512 slots are deleted/`$E5`), 24 of them `.COM` programs (`ALLOC`, `ASM`, `ASSIGN`, `BCOMPILE`, `COMPARE`, `COPY`, `DBASIC`, `DBASICMP`, `DEBUG`, `EDIT`, `MKCOM`, `NVSET`, `PASCAL`, `PRPDRV`, `PRUN`, `RUN`, `S19`, `SEDIT`, `SPSC`, `ULTIMA`, `WYRMHOLD`, `XMR`, `XMS`, `ZIP`), plus `MICROCHE`, `TREK.BAS`, `ZIPTEST.Z3`, `ZORK1.Z3`, `DBASICMA`. This corrects an earlier report (`mc-0sj`) that checked LBA 60 instead of LBA 256 and concluded the disk was empty; the correction was already made in `plans/pc6502-emulator-milestones/artifact-verification.md` at commit `f48cfbe` (2026-07-02) but had not yet been reflected here. See §10.5 for the resulting gate-test coverage gap. → `plans/pc6502-emulator-milestones/artifact-verification.md` §1.1

**Image format:** headerless, sector-zero-first. No container header adjustment in the code. → DOS §4.4

**Staging payload layout** (written to XT-IDE via Supermon `W` or equivalent):

| Staging address | Content |
|---|---|
| `$0800-$085E` | PC6502 loader (95 bytes) |
| `$1000-$37FF` | DOS/65 OS (linked at `$B800`; occupies `$B800-$D870` after relocation) |
| `$4000-$5FFF` | Banked driver (linked at `$C000`; occupies `$C000-$D5B0` after relocation) |

Sparse-gap fill byte **unknown** on real hardware. **[2026-07-02]** Zero-fill is what the emulator implements: `DiskImage::blank()` and the auto-extend path in `write_sector()` both zero-fill (`emulator/src/disk.rs:20,40`), matching this document's own recommended default. → DOS §3.3

### 7.3 SIM jump table (PC6502 build, current listing)

Task-0 addresses. Use `table_base + offset` when the OS is rebuilt. → DOS §5.2

| Address | Offset | Function |
|---|---|---|
| `$CBDC` | +0 | Cold boot |
| `$CBDF` | +3 | Warm boot |
| `$CBE2` | +6 | Console status (→ `$00` / `$FF`) |
| `$CBE5` | +9 | Console input (blocking) |
| `$CBE8` | +12 | Console output |
| `$CBEB` | +15 | Printer output |
| `$CBF4` | +24 | Home (set track 0) |
| `$CBF7` | +27 | Select disk (A=drive → A/Y=DCB pointer) |
| `$CBFA` | +30 | Select track |
| `$CBFD` | +33 | Select sector |
| `$CC00` | +36 | Set DMA address |
| `$CC03` | +39 | Read logical sector (A=`$00` success) |
| `$CC06` | +42 | Write logical sector (A=`$00` success) |
| `$CC09` | +45 | Printer status (returns `$01` always) |
| `$CC0C` | +48 | Read clock (stub: X=`$80`; time bytes not established) |
| `$CC0F` | +51 | Sector translate (no-op) |

### 7.4 Banked dispatcher (`$FFF0` far call) → DOS §5.3, MEM §7.1

`farfunct` at zero-page `$32` selects the function. ROM switches to task 1, calls dispatcher `$C000`, returns to task 0.

Selected key functions:

| farfunct | Function |
|---|---|
| 0-3 | Default console output/read/blocking/status (redirected via `CONSOLE` selector) |
| 4-8 | 6551 serial write/read/blocking read/status/init |
| 19-23 | Mapped video + Multi-I/O keyboard (VIDEO ROM's default group, selector `$13`) |
| 60-62 | XT-IDE init / read 512-byte sector / write 512-byte sector |
| 63-65 | CH375 init / read / write |
| 66-68 | Floppy init/read/write (no-op in this build) |

### 7.5 Console selection → DOS §7.1

`CONSOLE` at zero-page `$3A` is a dispatcher base, not an enum.

| Selector | Functions | Hardware |
|---|---|---|
| `$04` | 4-7 | On-board 6551 UART |
| `$09` | 9-12 | Dual ESP |
| `$0E` | 14-17 | ESP video + Multi-I/O keyboard |
| `$13` | 19-22 | Mapped video + Multi-I/O keyboard (VIDEO ROM default) |

Minimum interactive configuration: selector `$04` (base ROM) — requires only the on-board UART. The VIDEO unattended path also requires mapped video hardware and a Multi-I/O keyboard interface.

### 7.6 Status and error behavior → DOS §6

- Storage operations: `A=$00` = success; nonzero = failure (usually `$FF`).
- PEM on failure: prints "BAD SECTOR" message; Return ignores and continues; any other key triggers warm boot.
- SIM cold boot ignores all device-init return values — absent devices are not fatal at boot.
- IDE BUSY/DRQ polling uses 16-bit counters, not wall-clock timers — the emulator does not need to reproduce elapsed time but must allow status to progress.
- Serial TX and blocking RX have no timeout; if the emulated UART never signals ready, the guest loops forever intentionally.
- The SIM read-modify-write for disk writes does not test the preliminary read result before writing; emulator fault injection should expose this, not repair it. → DOS §6.2

---

## 8. Cross-document conflict registry

All conflicts identified across the five source investigations, with agreed resolutions for emulator work.

| # | Topic | Conflict | Agreed resolution |
|---|---|---|---|
| C1 | RAM endpoint | Board prints `$0000-$E000`; I/O starts at `$E000` | Normalize default RAM to `$0000-$DFFF`; give I/O decode priority at `$E000`. **Applies everywhere.** → HW §9, MEM §3.1, ROM §8, DOS §implicit |
| C2 | Board CPU identity title | Board document heading says `6809PC` | Stale copy/paste error; body, firmware, and listings are all 6502-specific. Emulate 6502. → HW §2, ROM §8 |
| C3 | ROM size: 4 KiB vs 8 KiB | Board says 4 KiB ROM; `rom.hex` decodes to 8 KiB | 8 KiB physical device holds two K1-selectable 4 KiB images; only one is CPU-visible at a time. Not a contradiction. → ROM §1, §3, §8 |
| C4 | HEX record addresses vs CPU addresses | `rom.hex` uses `$6000-$7FFF`; firmware links at `$F000` | Programmer-image bank locations, not CPU addresses. Apply `cpu_address = $F000 + (image_address - bank_start)`. → ROM §4 |
| C5 | MMU notation `$xFE0` vs `$EFxx` | Older firmware comments use `$xFE0`; current PC6502 firmware hardcodes `$EFxx` | The I/O page is at physical page `$0E`; its logical location follows the active task map. `$EFxx` is the confirmed software contract for the default task-0 map. Do not invent relocatable I/O behavior beyond what `$EFE7` (current I/O page) implies. → HW §9, MEM §5 preamble |
| C6 | 512 KiB RAM vs physical pages above `$7F` | Board says 512 KiB RAM; MMU can address page `$F8` | Separate physical decode space from installed RAM. Pages above `$7F` may select expansion hardware or a hole. Do not silently back them with RAM. → HW §5, MEM §2.1 |
| C7 | ACIA: 16-byte allocation vs 4 bytes used | Board allocates `$EF80-$EF8F`; firmware uses only `$EF84-$EF87` | Implement exactly `$EF84-$EF87` first; make wider mirroring a later evidence-based option. → HW §9, IO §UART decode |
| C8 | Video card: 32 KiB claim vs offsets beyond one 32 KiB span | Driver calls it 32 KiB; documented modes reference offsets through `$BFFF` within the mapped window, which could exceed 32 KiB from page `$F8` | Only pages `$F8` and `$F9` proven by firmware. Do not allocate all `$F8-$FF` to video. Treat video memory layout as partially specified and optional. → HW §6.2 (conflict), MEM §2.3 |
| C9 | ESP variant in `rom.hex` | ESP listing/map exist; the Makefile line that would package ESP is commented out | `rom.hex` contains base + VIDEO. ESP is not packaged and has no standalone binary. → ROM §3.2, HW §8, DOS §3.1 |
| C10 | ACIA programmed reset vs command register bits | Firmware comment says software reset does not affect any command bits; W65C51S manual says it clears bits 4:0 | Boot writes `$0B` to command immediately after the reset write, making this irrelevant to the boot path. Implement classic-compatible reset (clear bits 4:0, preserve parity bits); note discrepancy. → IO §UART command |
| C11 | ACIA control register: programmed reset vs register retention | Firmware comment implies control register low bits cleared by software reset; W65C51S reference leaves control unchanged during programmed reset | Boot overwrites control immediately after the sequence. Implement W65C51S reference behavior (control unchanged by programmed reset); note discrepancy. → IO §UART control |
| C12 | VIDEO bank and serial requirement | Bank is named "VIDEO" and sets console selector `$13`; but ROM still initializes the UART and both banks call `SERIALINIT` | The VIDEO bank emits serial output during banner; a working UART is required for this bank even when console selector is `$13`. Do not remove the UART for VIDEO. → ROM §6.3, DOS §7.1 |
| C13 | RTC chip identity | Board says "battery backed RTC"; firmware model matches Epson RTC-72421/72423 | Implement RTC-72421/72423 register model. Document as "compatible model — physical part unconfirmed." → IO §RTC identity |
| C14 | RTC logical fields 5 vs 6 | Driver API maps both X=5 and X=6 to raw offsets `$A-$B` (year) | Source mismatch: X=6 is explicitly decremented to 5 before address calculation. Both fields read/write year. Weekday (offset `$C`) is never set by PC6502 firmware. → IO §RTC driver API |
| C15 | Task-0 alias defect | Firmware says "some boards" duplicate edit writes into task 0 | Not a universal hardware behavior. Default: normal (only selected edit task). Optional named compatibility mode. → MEM §8, HW §9 |
| C16 | `$EFE4` bit 7 interpretation | hardware-spec says "lower six bits meaningful" (read active task); memory-mmu references V1.1 schematic showing bit 7 = MMU-enable status | The V1.1 schematic is the direct hardware evidence. Implement: bits 5:0 = active task, bit 7 = MMU-enable status, bit 6 = unassigned. → MEM §5.2 [E3 note] |

---

## 9. Ranked unknowns by implementation risk

### Priority 0 — Required for cycle-credible baseboard emulation

| Risk # | Unknown | Why it blocks | Implementation status [2026-07-02] |
|---|---|---|---|
| R0.1 | CPU exact part (NMOS 6502 vs CMOS 65C02 variant and vendor) | Undocumented opcodes, decimal-mode behavior, reset cycle count, interrupt timing all vary. → HW §10, ROM §7.3 | **Open — config field is dead.** `cpu_subtype` (`Nmos6502`/`Cmos65c02`) exists in `config.rs:95` but is never read anywhere in `cpu/opcodes.rs`; both variants execute identically today. Hardware fact still unconfirmed. |
| R0.2 | CPU oscillator frequency | All timing-sensitive behavior (UART baud clock, RTC timebase, startup delay, ATA wait-state adequacy) is unknown. → HW §7.1 | **Open — placeholder only.** `cpu_hz` field exists (`config.rs:99-100`); its own doc-comment states "not used for timing accuracy, exposed for future use." No timing-sensitive behavior is cycle-driven yet. |
| R0.3 | MMU power-on map contents | Map SRAM is volatile; contents before BIOS writes tasks 0/1 are indeterminate. Emulator must never promise specific values. → MEM §6.1, §13 | **Resolved (tooling).** `mmu_power_on_fill` (Zero/Random/Fixed) is implemented and consulted in `Mmu::new()` (`mmu.rs:26-42`). Note: `Random` is actually a fixed byte `0xA5`, not true randomness — the code comment says this is intentional, for reproducible tests. |
| R0.4 | Physical default K1 ROM bank | The emulator cannot know which bank was shipped or configured. Must expose a user-selectable configuration. → ROM §7.3, §9 | **Resolved (tooling).** `rom_bank` (Base/Video) is implemented and consulted (`rom.rs`, `emulator.rs::Machine::new`). Bank is fixed for the process lifetime, set at construction — matches this document's own "applied at reset, not at runtime" requirement (§5.6 item 2). No hot bank-switching exists. |
| R0.5 | Open-bus read value for unmapped addresses | Reads to `$EFA0-$EFCF`, `$EFF0-$EFFF`, unassigned MMU offsets, and absent physical pages return unknown data. Must not silently return `$00`; should use a named, configurable policy. → MEM §9.4, §13, HW §7.3 | **Resolved (tooling).** `open_bus.value` (default `0xEA`) is threaded through `Bus`, `Mmu`, `XtIde`, `Peripherals` and covered by gate test `open_bus_at_efa0`. Caveat: `$EFE6`/`$EFE7` reach this value only by falling through an unimplemented match arm, not by deliberate policy — see R1.5/R1.6. |
| R0.6 | Shadow-address strap setting (P1 `SHADOW ADDR`) | The V1.1 schematic exposes a configuration input that selects the high address bits used while the MMU is disabled. The local repository does not record the installed jumper position. Default must match the firmware-compatible low-page assignment. → MEM §2.2, §13, §15 | **Open — config field is dead.** `shadow_addr_low` exists (`config.rs:116`, default `true`) but is never read outside `config.rs` — confirmed by repo-wide search. The only behavior the emulator has is the hardcoded low-page assignment (correct default), but it is not actually gated by this flag. |
| R0.7 | ROM/I/O overlay precedence over arbitrary MMU mappings | It is confirmed that task-0 logical pages `$E` and `$F` identity-map to physical I/O and ROM, and that this works correctly. Whether the board always forces I/O/ROM decode regardless of the MMU mapping at those physical pages is **unknown**. Only the identity-map case is safe to assume. → MEM §3.2, ROM §7.2, §7.3 | **Superseded by a hardcoded implementation choice.** `io_rom_always` exists in config (`config.rs:118-122`) specifically for this OQ but is never read by `bus.rs`/`mmu.rs` — confirmed dead. Actual behavior: `Bus::read`/`write` (`bus.rs:57-83`) decode `$E000-$EFFF` and `$F000-$FFFF` unconditionally by CPU address *before* ever consulting the MMU; `translate_addr()` is only called for addresses below `$E000`. This makes the identity-map case always correct, but a task that remaps a *low* logical page to physical `$0E`/`$0F` would alias into plain RAM rather than I/O/ROM (`translate_ram`, `bus.rs:85-94`, does not exclude those physical pages). The "only identity-map case is safe" caveat in this document remains true of the emulator, not just of the hardware. |

### Priority 1 — Required for full correct operation

| Risk # | Unknown | Why it matters | Implementation status [2026-07-02] |
|---|---|---|---|
| R1.1 | ACIA variant and reference clock | Baud-rate timing, transmit-ready silicon behavior (W65C51N errata), IRQ output wiring. → IO §UART, DOS §7.2, HW §7.1 | **Open — config field is dead.** `acia_variant` is stored as `_variant` in `acia.rs:20` (underscore-prefixed — Rust's own convention for "deliberately unread"), set once from config and never consulted. No baud timing or W65C51N-errata modeling exists. |
| R1.2 | ACIA address decode and mirroring within `$EF80-$EF8F` | Software that probes or uses addresses beyond `$EF84-$EF87` will get wrong results. → IO §UART decode, HW §9 C7 | **Resolved as recommended.** `bus.rs` wires exactly `$EF84-$EF87` to the ACIA; the rest of `$EF80-$EF8F` falls to the generic open-bus catch-all. Matches this document's "implement exact offsets first" guidance; mirroring remains unimplemented, consistent with "unknown." |
| R1.3 | RTC chip identity and interrupt wiring | HOLD/BUSY timing, periodic output connection to IRQ/NMI, power-on register state. → IO §RTC, HW §6.1 | **Register model implemented, with a bug; IRQ wiring still open.** RTC-72421/72423 model exists in `rtc.rs` as recommended, but see the field-mapping and STOP-bit findings in §10.4 — both concrete implementation bugs, not hardware unknowns. No IRQ/NMI connection from the RTC exists at all (control-register IRQ/period bits are stored but have no observable effect). |
| R1.4 | Modem-input defaults (CTS, DCD, DSR) | CTS forces high by a board jumper; a wrong default can deadlock UART transmit. → IO §UART init, DOS §7.2 | **Partially resolved.** `acia_cts_default` (default `true`) is implemented and gates TX (`acia.rs:40,64-66`) — the CTS half of this OQ is done and matches the recommended default. DCD/DSR are not modeled at all, not even as stub status bits. |
| R1.5 | `$EFE6` terminal-count pulse width, polarity, and clearing | Firmware reads it in the IDE driver path; effect on real hardware is not established. → MEM §5.2, IO §CPU-visible decode | **Open — not implemented at all.** `Mmu::io_read` (`mmu.rs`) has no match arm for offset `0x16`; it falls through to the generic open-bus default with no side effect. Stronger than "unknown on hardware": the emulator has made no choice here yet. |
| R1.6 | `$EFE7` I/O-page readback under nondefault shadow straps | The registered value is confirmed; semantics under alternate configurations are not. → MEM §5.2, §13, MEM §15 | **Open — not implemented at all.** Same gap as R1.5: no match arm for offset `0x17` in `Mmu::io_read`; falls through to open-bus instead of returning bits `3:0` = current I/O page as documented in §4.2. |
| R1.7 | MMU active-task alias defect — which board revisions have it | Without knowing revision, implementing as off-by-default is the only safe choice. → MEM §8, §13 | **Consistent with "default off," but there is no mode to turn on.** `mmu.rs` has exactly one code path (edit window always addresses `setup_task`); no alternate "also writes task 0" behavior exists anywhere, and no config flag selects it. Slightly stronger than the "off-by-default, optional named mode" this document calls for — the optional mode itself doesn't exist yet. |

### Priority 2 — Required for optional-peripheral compatibility

| Risk # | Unknown | Why it matters |
|---|---|---|
| R2.1 | Slot connector pinout, voltage, clock, wait states | Cannot model slot decode timing without this. → HW §7.2, IO §slots |
| R2.2 | IRQ jumper position mapping and polarity | Device IRQs are all polled in firmware; wiring unknown. → HW §7.3, IO §interrupt matrix |
| R2.3 | Video card full memory map beyond pages `$F8-$F9` | 32 KiB claim conflicts with observed access patterns. → HW §6.2 C8, MEM §2.3 |
| R2.4 | CH375 vs CH376 revision and `$E261` exact pin state behavior | Command-set minor differences; timeout return defect observed. → IO §CH375, DOS §6.4 |
| R2.5 | ESP endpoint FIFO depth and processing latency | Only handshake bits (BSY/RDY) are confirmed. → IO §ESP |
| R2.6 | Multi-I/O mouse port, IRQ routing, remaining status bits | Keyboard interrupts disabled; rest unconfirmed. → IO §Multi-I/O |
| R2.7 | ATX power-control and reset-switch electrical details | Relevant only for accurate power-cycle emulation. → HW §3, §7.4 |

### Hardware measurements that would remove the highest-risk unknowns

1. Identify installed CPU part from markings and confirm with undocumented-opcode probes.
2. Measure CPU oscillator frequency with a logic analyzer.
3. Record P1 shadow-address and K1 ROM-select jumper positions on a known-working PC6502.
4. Read `$EFE4` with several active tasks and both enable states to confirm bit 7 and bit 6.
5. Read `$EFE3`, `$EFE5`, and `$EFE8-$EFEF` after controlled preceding bus values.
6. Observe CTS, DCD, and DSR pin levels after power-on with no DTE connected.
7. Test map writes to task 1 while monitoring task 0 entries to identify the alias defect.
8. Map absent physical pages (`$80`, `$F0`, `$FF`) and record bus response.
9. Map logical page `$F` to a nondefault physical page and observe vector fetch behavior. → MEM §15

---

## 10. Emulator-facing implementation checklists

### 10.1 Memory and physical decode checklist

*Status column added 2026-07-02; checked items verified directly against `emulator/src/bus.rs` and `mmu.rs` at commit `0322d3c`.*

- [x] Separate 64 KiB logical CPU space from 20-bit (1 MiB) physical decode space. — `Bus` stores CPU-facing `u16` addresses and a separate `Vec<u8>` physical RAM array; `translate_ram()` bridges them (`bus.rs:85-94`).
- [x] Map 512 KiB SRAM to physical `$00000-$7FFFF`; do not silently extend RAM to cover all 256 pages. — `ram: vec![0u8; 512*1024]` (`bus.rs:46`); reads/writes past `ram.len()` return open-bus / are discarded (`bus.rs:63-67,77-79`).
- [ ] Give physical I/O overlay (page `$0E`) and ROM overlay (page `$0F`) priority over hidden SRAM for read and write. — **Only true for the identity-mapped default.** `Bus::read`/`write` hardcode CPU-address-range decode for `$E000-$FFFF` ahead of any MMU lookup, and `translate_ram()` does not exclude physical pages `$0E`/`$0F` for the below-`$E000` path. A task that remapped a low logical page to physical `$0E`/`$0F` would alias into RAM instead of I/O/ROM. See R0.7 in §9.
- [x] ROM writes must not update hidden SRAM. — `0xF000..=0xFFFF => {}` is a hard no-op in `Bus::write` (`bus.rs:74`).
- [x] Unclaimed physical reads use a named, configurable open-bus policy (not silently `$00`); unclaimed writes are discarded. — `open_bus.value` (default `0xEA`), threaded through `Bus`/`Mmu`/`XtIde`/`Peripherals`. Exception: `$EFE6`/`$EFE7` reach this value by unimplemented-arm fallthrough rather than deliberate policy — see §10.2.
- [x] Physical aliases (two logical pages mapping the same physical page) must be coherent. — Both logical pages compute the same physical index into the single backing `Vec<u8>`; coherency is automatic, not separately implemented.
- [x] Document the shadow-address strap assumption; default to firmware-compatible low-page (physical page = logical page number, I/O = `$0E`, ROM = `$0F`). — The default behavior is correct and is what the emulator actually does. Caveat: `shadow_addr_low` (the config field meant to make this selectable) is declared but never read anywhere outside `config.rs` — see R0.6 in §9. → MEM §11, §2.2

### 10.2 MMU checklist

*Status column added 2026-07-02, verified against `emulator/src/mmu.rs` at commit `0322d3c`.*

- [x] Model 64 tasks × 16 page entries × 8-bit selectors = 1,024 bytes of map SRAM. — `map: [u8; 1024]` (`mmu.rs:18`).
- [x] Initialize: MMU enable = 0, active task = 0 at hardware reset. Do not promise reset map contents. — `Mmu::new()` (`mmu.rs:36-42`); fill value per `mmu_power_on_fill` policy (R0.3, resolved).
- [x] Edit window (`$EFD0-$EFDF`) usable while MMU is disabled. — `io_read`/`io_write` offsets `0x00-0x0F` never check `enabled`.
- [x] `$EFE0` write: mask to 6 bits; apply new active task at end of write bus cycle. — `val & 0x3F` (`mmu.rs:100`). The emulator is not cycle-stepped at the bus-signal level, so "end of write bus cycle" isn't separately modeled; the new value is visible from the very next access, which is the observable-behavior equivalent.
- [x] `$EFE1` write: mask to 6 bits; no documented reset value. — `mmu.rs:104`.
- [x] `$EFE2` write: bit 0 only; next bus cycle uses new enable state. — `val & 0x01 != 0` (`mmu.rs:108`).
- [x] `$EFE4` read: bits 5:0 = active task, bit 7 = enable status, bit 6 = unassigned. — `mmu.rs:82-85`; bit 6 reads 0 always since `active_task` is masked to 6 bits at write time.
- [ ] `$EFE6` read: produce an observable TC event; do not depend on read data value. — **Not implemented.** No match arm for offset `0x16`; falls through to the generic open-bus default (`mmu.rs:86`) with no side effect at all. See R1.5 in §9.
- [ ] `$EFE7` read: return documented I/O-page default in low 4 bits; document upper bits. — **Not implemented.** No match arm for offset `0x17`; same open-bus fallthrough. See R1.6 in §9.
- [x] Unassigned offsets (`$EFE3`, `$EFE5`, `$EFE8-$EFEF`): use open-bus policy; writes are no-ops. — Falls through to `_ => self.open_bus` (read) / `_ => {}` (write). Note this is the same fallthrough arm that (incorrectly, for now) also catches `$EFE6`/`$EFE7`.
- [ ] `SETPAGE` (`$FFF6`) edge cases: see §4.5 — out-of-range X is not safe. — N/A to the Rust emulator directly: `SETPAGE` is BIOS 6502 code executed by the CPU core, not separate emulator logic. Satisfied as long as the CPU core and MMU register model are correct (gate-tested via M2/M4); no dedicated edge-case handling exists or is needed beyond that.
- [ ] Task-0 alias defect: default off; implement as named optional mode. — **Default-off is correct** (no alias write-through exists), but there is no config flag or code path implementing the optional mode at all — it isn't just "off," it's absent. See R1.7 in §9. → MEM §11, §12

### 10.3 ROM and reset checklist

*Status column added 2026-07-02. All items below are gate-tested at HEAD (`cargo test` with `PC6502_ROM_HEX`/`PC6502_DISK_IMG` set: 13/13 non-ignored pass).*

- [x] Accept `rom.hex` as 8 KiB Intel HEX; extract both 4 KiB banks. — `rom.rs::parse_intel_hex` / `Rom::load_hex`.
- [x] Expose bank selector (K1 equivalent) as machine configuration applied at reset. — `rom_bank` config (R0.4, resolved); fixed for process lifetime, set at construction.
- [x] Map only the selected bank to `$F000-$FFFF` before reset vector fetch. — `Rom::read` matches on `self.bank` (`rom.rs:38-44`).
- [x] Verify vector bytes: base `$F059`/`$F000`/`$F043`; VIDEO `$F060`/`$F000`/`$F04A`. — Confirmed behaviorally (not re-verified byte-for-byte this pass) via `m1_supermon_prompt` and `m3_real_rom_video_uart_banner` passing against real `rom.hex`.
- [x] Reset order: MMU disabled → vector fetch from ROM → BIOS initializes MMU → MMU enabled. — `Cpu::reset()` only fetches the vector (`cpu/mod.rs:35-44`); `Mmu::new()` starts disabled; enabling happens as ordinary 6502 execution of BIOS code, gate-tested via M2.
- [x] Common reset path requires: writable zero page, stack, `$0300`, UART init, MMU init. — Implied by M1/M3/M4 gate tests reaching Supermon/VIDEO banners and the DOS/65 prompt.
- [x] Base bank: deliver `BRK` so Supermon is reached; `$FFF0`/`$FFF3`/`$FFF6` stubs functional. — `m1_supermon_prompt` (real ROM).
- [x] VIDEO bank: XT-IDE reads LBAs 0-59 into `$0800-$7FFF`; jump to `$0800` on success; error path on any failure. — `m3_video_boot_and_60_sectors`, `m3_real_rom_video_uart_banner`.
- [x] Preserve ROM visibility through all BIOS-observed task maps. — Confirmed via M2/M4 gate tests (task-1 preserves logical page `$F`).
- [x] Treat hot bank switching, arbitrary page-F MMU remapping, and K1 default as unknown/configurable. — Consistent: no hot bank-switching exists (`Rom.bank` is set once at construction and never mutated); K1 default is the `rom_bank` config. → ROM §9

### 10.4 Peripheral I/O checklist

*Status column added 2026-07-02, verified against `emulator/src/{acia,rtc,xt_ide,peripherals}.rs` at commit `0322d3c`.*

- [x] **UART:** implement `$EF84-$EF87` exactly; decode no wider without evidence. — `bus.rs:110,134`.
- [x] **UART:** TDRE (bit 4) must eventually become 1; RDRF (bit 3) set by injected receive data, cleared by RDR read. — `acia.rs::update_status`; TDRE is unconditionally set every update.
- [x] **UART:** programmed reset `$00 → $EF85` follows classic-compatible behavior (clears command bits 4:0, leaves control unchanged). — `acia.rs:69-75`, mask `0b1110_0000` preserves bits 7:5, control register untouched. Resolves conflicts C10/C11 (§8) as implemented fact, not just agreed resolution.
- [ ] **UART:** command `$0B` → no IRQ, TX enabled, RTS low; control `$1E` → 9600 baud, 8N1. — Command/control registers are stored raw (`acia.rs`: `2 => self.command = val`, `3 => self.control = val`) but their bit content is never behaviorally interpreted beyond the reset-time mask above — there is no baud-rate, RTS-output, or IRQ-enable modeling to interpret `$0B`/`$1E` against. Consistent with R1.1/R0.2 (no ACIA-variant or timing model exists yet); not a regression, just unimplemented.
- [x] **UART:** do not discard bit 7 or NUL at the device level (firmware API filters these, not the ACIA). — `rx_queue: VecDeque<u8>` stores full bytes; no masking at the device layer.
- [ ] **UART:** expose CTS/DCD/DSR as configurable inputs; default virtual CTS asserted. — CTS done (`acia_cts_default`, default `true`, gates TX at `acia.rs:64-66`). DCD/DSR are not modeled at all — not even present as status-register stub bits. See R1.4 in §9.
- [ ] **RTC:** implement RTC-72421/72423 register model; low nibble only for data registers. — **Bug: register offsets are shifted starting at `$EF96`.** The one-BCD-digit-per-register mechanism itself is right (`val & 0x0F` / `regs[idx] & 0x0F`), but `Rtc::populate_from_unix()` (`rtc.rs:58-75`) writes weekday to raw offset `0x06` and then Day/Month/Year one register late each (Day at `0x07-0x08` instead of `0x06-0x07`, Month at `0x09-0x0A` instead of `0x08-0x09`, Year at `0x0A(partial)-0x0C` instead of `0x0A-0x0B`), leaving offset `0x0C` written with year-tens data instead of standing unused. Verified against firmware ground truth: `bios_rtc.asm`'s `RTC_READ`/`RTC_WRITE` (`PC6502_firmware_source/bios_rtc.asm:22-93`) address raw offset `2×field`, with logical field 6 explicitly decremented to field 5 (`CPX #6 / BNE / DEX`) — giving raw layout `$0-1`=Sec, `$2-3`=Min, `$4-5`=Hour, `$6-7`=Day, `$8-9`=Month, `$A-B`=Year (fields 5 and 6 alias), `$C`=Weekday (never addressed by the driver). This matches §6.2's existing raw register table exactly, which remains correct — it is `rtc.rs` that deviates from it. **The M6 gate test does not catch this**: `m6_rtc_config_gate.rs`'s `rtc_fixed_matches_epoch` asserts `bus.read(0xEF96) == 4` labeled `"weekday (Wed=4)"`, i.e. it encodes the shifted (wrong) layout as the expected result rather than checking against the documented offsets. Net effect: firmware reading the day-of-month, month, or year via the documented raw offsets gets the wrong BCD value; only hours/minutes/seconds are correct. This is an emulator bug, not a hardware unknown — the fix is to shift the five writes in `populate_from_unix` down by one register, moving weekday to `0x0C`.
- [ ] **RTC:** reproduce the exact control-register write sequence (`$02`/digits/`$00`/`$00`/`$01`/`$05`/`$04`) with correct side effects. — **Bug: STOP is checked on the wrong bit.** §6.2's own register table puts STOP at bit 1 (`0x02`) of `$EF9F`; `Rtc::write()` instead sets `self.stopped` from bit 3 (`val & 0x08`, `rtc.rs:41-43`), which this document's own table calls TEST. None of the documented driver sequence values (`$01`, `$02`, `$04`, `$05`) ever set bit 3, so `self.stopped` never becomes true via the real firmware sequence — the STOP mechanism is effectively dead code. Low practical impact today, since `read()` recomputes fresh from the clock policy on every access regardless of `stopped` rather than freezing a ticking register set, but it means the emulator does not actually model "read while stopped" vs. "read while running" as two different states as the hardware sequence implies it should.
- [x] **RTC:** expose register-initialization policy (persistent, host-time, or fixed epoch) — do not assume power-on state. — `rtc_policy` (Host/Fixed/Epoch) implemented in `rtc.rs::update_from_policy`. Minor note: `Fixed` and `Epoch` currently produce identical behavior (`self.epoch` shown statically, never advanced from wall-clock elapsed time) even though the field's own doc-comment describes `Epoch` as "advance from a configured start time" — that advancing behavior is not yet implemented, so the two policies are presently indistinguishable.
- [ ] **RTC:** weekday register `$C` is never set by PC6502 firmware; reflect this in test coverage. — Still true of the *firmware*. Not true of the *emulator*: as a side effect of the offset-shift bug above, `rtc.rs` currently writes year-tens data into offset `0x0C` on every clock update. Fixing the shift bug (above) also fixes this — `$0C` should end up genuinely unwritten/inert, matching this line.
- [x] **XT-IDE:** 256 low/high word pairs = 512 bytes per transfer; BSY/DRQ/ERR drive all wait loops. — DRQ/ERR are modeled and drive the transfer state machine (`xt_ide.rs::read_inner`/`execute_command`). Note: the `BSY` constant (`0x80`) is declared but never actually asserted anywhere — all commands complete instantly with `BSY=0`, so busy-polling loops always see "not busy" immediately. A reasonable simplification given no timing model exists (R0.2), not a correctness bug for a firmware that only waits for BSY to clear.
- [x] **XT-IDE:** tolerate probe writes `$FF` and `$00` across `$E300-$E330` without treating them as fatal. — Unknown offsets silently discarded (`xt_ide.rs:167`); gate-tested via M3.
- [x] **XT-IDE:** firmware SET FEATURES `$EF` feature `$01` must succeed or auto-proceed. — `execute_command`'s `0xEF` arm clears to `DRDY`, no `DRQ`/`ERR` (`xt_ide.rs:212-218`).
- [x] **Optional cards:** absent devices must not crash the emulator; expansion window reads return the configured open-bus value. — Refined beyond generic open-bus, and better for it: CH375 returns a fixed `0x00`, and ESP returns specifically-chosen values (`$E102=0x09`, `$E100/$E101=0x01`, `peripherals.rs:7-21`) reverse-engineered against the DOS/65 driver's own bit-test polling loops so absent-device probing exits immediately instead of spinning through 65K-cycle timeouts. `$E102=0x09` (ESP0 RDY=1, ESP0 BSY=0, ESP1 RDY=1, ESP1 BSY=0) is independently consistent with this document's own §6.3 bit table for that register. Multi-I/O keyboard self-test (`$AA→$55`) is also modeled (`peripherals.rs:52-64`).
- [x] **Interrupts:** default UART, RTC, keyboard, and storage configuration must not spuriously assert CPU IRQ. — True, but by absence rather than by design: no code path in `acia.rs`/`rtc.rs`/`mmu.rs`/`xt_ide.rs` asserts an IRQ signal at all yet (status bit 7 in `acia.rs` is never set). "No spurious IRQs" and "no IRQ delivery mechanism exists yet" are currently the same fact. → IO §required tests

### 10.5 DOS/65 boot and services checklist

*Status column added 2026-07-02. Items below marked "gate-tested" pass at HEAD with real `rom.hex`/`disk.img` (`PC6502_ROM_HEX`/`PC6502_DISK_IMG` set).*

- [x] Reset reaches `$F000`; common init completes without device IRQs. — Gate-tested (M1/M3/M4).
- [x] VIDEO bank issues sequential ATA reads LBA `0..59`; all 60 must succeed before jump to `$0800`. — Gate-tested (`m3_video_boot_and_60_sectors`, `m3_real_rom_video_uart_banner`).
- [x] Loader produces distinct task-0 and task-1 contents at logical `$C000-$DFFF`. — Gate-tested (M4, physical `$B800`/`$10000`/`$11FFF` assertions).
- [x] Far call `JSR $FFF0` executes dispatcher `$C000` in task 1; returns to task-0 caller with A preserved. — **Newly gate-tested since this document's synthesis**: `m4_real_boot_far_call_and_sim_init` (added in `f8ffd6c`) exercises this against the real ROM/disk artifacts, resolving what `artifact-verification.md` (REQ-M4-3) had listed as "ready to implement."
- [x] UART status bit 4 permits output; status bit 3 controls input availability; no UART IRQ needed for DOS prompt. — Gate-tested (boot-to-`A>`-prompt tests pass with no IRQ modeling present at all).
- [x] Disk LBA mapping: logical (track=0, sector=0) → LBA 0 byte 0; sector 1 → LBA 0 byte 128; sector 4 → LBA 1; track 1, sector 0 → LBA 16; slice 1 → add `$4100` to all LBAs. — Gate-tested at the slice boundary (`m5_disk_io_gate` drive-B isolation section).
- [ ] 128-byte logical write changes only the selected quarter of the 512-byte host sector. — N/A to the Rust emulator: the XT-IDE model only deals in 512-byte sectors; the 128-byte quarter-sector read-modify-write is entirely DOS/65 SIM firmware behavior (`simrbc.asm`) executing on the emulated CPU core, with nothing device-side to implement separately.
- [ ] Failed ROM disk read reaches monitor error path, not `$0800`. — **Coverage gap, not a known bug.** No current gate test injects a bad sector during the ROM's own 60-sector boot read to confirm the error path is reached instead of `$0800`.
- [ ] Failed DOS read: nonzero A → PEM bad-sector prompt; does not silently continue. — **Coverage gap.** `m5`'s bad-sector tests (`inject_bad_sector` + direct READ/WRITE SECTORS) exercise the XT-IDE register contract directly; none currently drive a DOS/65-level command to a bad sector and assert the PEM prompt text appears.
- [x] Drives A/B access nonoverlapping slices of the same XT-IDE master. — Gate-tested (`m5_disk_io_gate` §1d).
- [x] Drives E-H return failure; DCB presence does not imply working storage. — **Newly gate-tested since this document's synthesis**: `m5_drive_e_failure` (added in `f8ffd6c`) resolves REQ-M5-4 from "ready to implement" to implemented.
- [x] SIM cold-boot init-call failures for optional devices do not prevent reaching the DOS prompt. — Gate-tested (absent-device stub tests + full boot to `A>`).
- [ ] SIM read-clock stub returns X=`$80`; full RTC time service is not available via this entry point. — Unchanged DOS/65 firmware behavior (`simrbc.asm` jump table); not exercised or altered by any reviewed change.
- [ ] **[2026-07-02, new item]** The shipped `emulator/disk_image/disk.img`'s CP/M directory is at LBA `0x0100` (256), not LBA 60, and is populated: 29 active files, 24 of them `.COM`. See §7.2 for the full file list and provenance. **Do not assume an empty disk when testing or implementing DOS/65 file operations against the stock image.** — This correction (from `plans/pc6502-emulator-milestones/artifact-verification.md` at commit `f48cfbe`) unblocked REQ-M5-1 (`DIR A:`) and REQ-M5-2 (`.COM` load) gate-test work: `m5_dir_listing` now exists and passes (REQ-M5-1 implemented), **but REQ-M5-2 does not** — `m5_com_load` in `emulator/tests/m5_disk_io_gate.rs` is still `#[ignore = "requires a disk image with at least one .COM file; stock disk has empty CP/M directory"]` (and a header comment on the same file repeats the LBA-60/empty claim), even though `artifact-verification.md` explicitly names `S19.COM` as a suitable minimal test candidate. This is a live, actionable gap for the next implementer, not merely a documentation staleness: the capability is untested despite the disk having what's needed to test it.

---

## 11. Evidence cross-reference

The following index maps each source investigation's evidence tags to the synthesizing sections of this document. Use these to trace a claim back to the firmware or board-document line that supports it.

| Source tag | Investigation reference | Used in this document |
|---|---|---|
| HW [E1] | `documentation/PC6502_system_documentation.md:1-24` — board identity, slots, jumpers, connectors | §2 (identity), §3 (decode), §6.3 (slots) |
| HW [E2] | `documentation/PC6502_system_documentation.md:28-41` — default memory and I/O maps | §3 (integrated map), §4 (MMU) |
| HW [E3-E15] | `PC6502_firmware_source/*.asm/lst/map/hex` — firmware provenance, BIOS sources | §4 (MMU), §5 (ROM), §6 (I/O), §7 (DOS) |
| MEM [E3] | Upstream `MMU-4.kicad_sch` V1.1 — 20-bit path, task latches, TC output, reset state | §4.2 (register table), §9 R0.6 |
| MEM [E4] | Upstream `memory-map.kicad_sch` V1.1 — SRAM, shadow decode, disabled-MMU path | §3.1 (precedence), §4 (MMU), §9 R0.6 |
| MEM [E5] | `PC6502_firmware_source/bios_pager.asm` — task maps, register descriptions, SETPAGE | §4 (MMU), §4.5 (SETPAGE), §4.6 (alias defect) |
| ROM [E2] | `PC6502_firmware_source/rom.hex:1-257` — 8 KiB HEX data | §5.1 (image), §5.2 (vectors) |
| ROM [E3] | `6502PCbios.asm:2-177,215-230` — BIOS source, reset flow, vectors | §5.4 (reset flow) |
| ROM [E5] | `PC6502_firmware_source/Makefile:1-10,26-33` — ROM build rules | §5.1 (bank structure), §8 C9 |
| ROM [E7] | `6502PCbios.lst` and `6502PCbiosvideo.lst` — exact reset bytes and vector bytes | §5.2 (exact vectors), §5.3 (stubs) |
| ROM [E8] | `bios_ide.asm:624-669` — VIDEO ROM IDE boot loop | §5.5 (auto-boot flow) |
| DOS [E4] | `loader.asm:8-86` — staging copy flow | §7.1 (boot flow), §7.2 (staging layout) |
| DOS [E7] | `DOS65_OS/dos65_os/simrbc.asm` — SIM API, DCB geometry, disk read/write | §7.2 (disk model), §7.3 (SIM table) |
| DOS [E12] | `bios_ide.asm:11-35,282-750` — XT-IDE contract, 60-sector ROM boot | §6.3 (XT-IDE), §7.1 (boot), §10.4 |
| IO [L2] | `bios_serial.asm:15-157` — UART registers, init, polling | §6.1 (UART) |
| IO [L3] | `bios_rtc.asm:11-180` — RTC base, nibble access, control sequence | §6.2 (RTC) |
| IO [L7-L10] | `bios_esp/ch375/ide/multi.asm` — expansion card register maps | §6.3 (expansion) |
| IO [D1] | WDC W65C51S data sheet | §6.1 (UART register detail) |
| IO [D2] | Epson RTC-72421/72423 application manual | §6.2 (RTC register detail) |
