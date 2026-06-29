# PC6502 Hardware Specification and Source Inventory

**Investigation date:** 2026-06-27  
**Scope:** Evidence available in the local `PC6502Emulator` repository snapshot  
**Purpose:** Establish the hardware baseline and evidence hierarchy for emulator implementation and the follow-on memory/MMU, ROM/reset, I/O, and DOS/65 investigations.

## 1. Confidence model

This document uses four labels:

- **Confirmed** — stated by the board documentation and corroborated by firmware, or directly exercised by generated machine-code listings.
- **Source-observed** — present in firmware or build artifacts, but not necessarily proven to match production hardware.
- **Inferred** — the most consistent interpretation of multiple sources; emulator code should keep the behavior configurable until hardware validation exists.
- **Unknown** — the repository does not contain enough evidence to specify the behavior.

The repository contains board-level notes and executable firmware evidence, but no schematic, PCB design, bill of materials, programmable-logic source, logic-analyzer capture, or existing emulator implementation [A1]. Consequently, this is an emulator-facing functional specification, not an electrical specification.

## 2. System identity

The target is a 6502-family, ATX-format computer referred to interchangeably as **6502PC** in the board document and **PC6502** in firmware/build identifiers. The board document claims 512 KiB RAM, a 4 KiB ROM window, a programmable MMU, a battery-backed RTC, a 6551 UART, and six “ISAish” slots [E1]. The BIOS identifies itself as the “6502 PC ATX SBC,” says it requires paged memory, and records a PC6502 port by Dan Werner dated 2025-12-06 [E3]. Treat “PC6502” and “6502PC” as the same target unless new hardware evidence distinguishes them (**inferred**).

The headings `# 6809PC` in the board document and “for 6809PC” in the ESP driver conflict with the surrounding 6502-specific content [E1, E10]. These appear to be stale copy/paste labels (**inferred**) and are not evidence for a 6809 CPU.

The exact CPU part is **unknown**. The sources establish a 6502 software target, but do not identify NMOS 6502 versus a particular CMOS 65C02 vendor/revision, nor document undocumented-opcode, decimal-mode, interrupt-timing, or reset-cycle behavior [E3, E19]. DOS/65 documentation mentioning either a 6502 or 65816 applies to several supported machines and does not prove that the PC6502 board accepts both [E20].

## 3. Baseboard component inventory

| Component | Status | Functional evidence and emulator consequence |
|---|---|---|
| CPU | **Confirmed: 6502 software target; exact part unknown** | The BIOS and the PC6502-specific DOS/65 build are 6502 sources [E3, E19]. Start with a documented 6502 core, but keep CPU subtype selection explicit until the physical part is identified. |
| Main RAM | **Documented: 512 KiB** | The only capacity statement is 512 KiB [E1]. The MMU page selector can express mappings outside that capacity, so page values must not automatically imply installed RAM [E5]. |
| Boot ROM | **Confirmed: 4 KiB CPU window; two selectable images in the supplied 8 KiB programming file** | CPU-visible ROM is `$F000-$FFFF`; firmware is linked at `$F000` and places vectors at `$FFFA-$FFFF` [E2, E3, E16, E17]. Jumper K1 selects one of two 4 KiB image regions, and `rom.hex` contains data records spanning `$6000-$7FFF` [E1, E15, E18]. |
| MMU | **Confirmed functional dependency** | Firmware describes 64 task contexts, each containing sixteen 4 KiB mappings, and actively programs task 0 and task 1 before enabling the MMU [E5]. Exact reset contents, physical address width, invalid-page behavior, and I/O/ROM override priority remain unknown. |
| Console UART | **Confirmed: 6551-compatible interface** | The board document names a 6551 UART and reserves `$EF80-$EF8F`; firmware uses data/status/command/control at `$EF84-$EF87` [E1, E2, E6]. The emulator must initially decode the four exercised addresses; whether the device is mirrored across the full 16-byte range is unknown. |
| RTC | **Confirmed: battery-backed RTC function; chip identity unknown** | The board document states battery backup and provides a non-rechargeable battery connector; firmware accesses a nibble-oriented register block at `$EF90-$EF9F` [E1, E2, E7]. Do not identify a specific RTC chip without schematic or package-marking evidence. |
| Expansion bus | **Confirmed: six slots described as “ISAish”** | Six slot connectors and six IRQ-assignment jumpers are named, and `$E000-$EF7F` is assigned as ISA I/O space [E1, E2]. “ISAish” does not establish IBM ISA electrical, timing, DMA, bus-mastering, or interrupt compatibility. |
| Power/reset and console connectors | **Documented, electrically unspecified** | The board notes list momentary ATX power and reset switches, a console serial connector, a TTL console connector, TTL connector power enable, and CTS-force-high jumper [E1]. Voltage levels, polarity, debounce, and ATX control sequencing are unknown. |

## 4. CPU-visible address-space baseline

The following is the safest initial decode model. It intentionally distinguishes literal documentation from normalization needed to remove contradictions.

| CPU address range | Initial interpretation | Confidence and evidence |
|---|---|---|
| `$0000-$DFFF` | RAM under the reset/default mapping | **Inferred.** The board note prints RAM as `$0000-$E000`, which overlaps the separately documented I/O start at `$E000`; treating the RAM endpoint as `$DFFF` is the only non-overlapping interpretation [E2]. MMU mapping can subsequently change the physical backing [E5]. |
| `$E000-$EF7F` | Expansion/“ISA” I/O | **Documented** [E2]. Firmware exercises optional devices at `$E100`, `$E260`, `$E300`, and `$E3E0`-based offsets [E8-E11]. Unclaimed-address behavior is unknown. |
| `$EF80-$EF8F` | ACIA allocation | **Documented allocation; partial decode confirmed** [E2, E6]. Firmware uses `$EF84-$EF87` only. |
| `$EF90-$EF9F` | RTC allocation | **Confirmed** [E2, E7]. |
| `$EFA0-$EFCF` | Open/reserved | **Documented as open** [E2]. Whether reads return floating-bus data, `$00`, `$FF`, or mirrored devices is unknown. |
| `$EFD0-$EFDF` | MMU task-map edit window | **Confirmed** [E2, E4, E5]. The selected setup task determines which sixteen page entries are edited. |
| `$EFE0-$EFEF` | MMU control/status allocation | **Confirmed allocation; only some offsets documented** [E2, E4, E5]. |
| `$EFF0-$EFFF` | Open/reserved | **Documented as open** [E2]. Read value and write effects are unknown. |
| `$F000-$FFFF` | 4 KiB boot ROM window | **Confirmed** [E2, E3, E16, E17]. |

The board note's `$0000-$E000` RAM endpoint is a specification defect, not a one-byte RAM region at `$E000` (**inferred**) [E2]. Emulator memory decoding should give the explicitly documented I/O region precedence at `$E000`.

## 5. MMU and physical-address model

Firmware says each task has sixteen 4 KiB entries, covering the complete 64 KiB logical address space, and that task IDs are 0-63 [E5]. The programmed register addresses are:

| Address | Firmware meaning | Evidence |
|---|---|---|
| `$EFD0-$EFDF` | Sixteen-entry edit window for the selected setup task | [E4, E5] |
| `$EFE0` | Write active task | [E4, E5] |
| `$EFE1` | Write task selected for editing | [E4, E5] |
| `$EFE2` | Write MMU enable (`0` disabled, `1` enabled) | [E4, E5] |
| `$EFE4` | Read active task, lower six bits meaningful | [E5] |
| `$EFE6` | Read “hit ISA TC bit”; semantics otherwise unknown | [E5] |
| `$EFE7` | Read current I/O page, lower four bits meaningful | [E5] |

On initialization, firmware first disables the MMU, builds task 1 as a one-to-one map except logical pages `$C` and `$D` map to physical pages `$10` and `$11`, then builds task 0 as a one-to-one map, selects task 0, and enables the MMU [E5]. The ordering is deliberate: a source comment warns that on some boards every edit-window write also affects task 0 [E5]. That warning implies hardware revisions or implementations with differing side effects; an emulator should preserve the normal selected-task behavior and track the task-0 alias as a compatibility option until physical hardware resolves it (**inferred**).

The main BIOS calls MMU initialization before normal console operation and temporarily switches to task 1 to reach the banked driver dispatcher at logical `$C000` [E3, E5]. The DOS/65 loader copies the OS to task-0 `$B800-$DFFF`, copies drivers to task-1 `$C000-$DFFF`, then returns to task 0 and jumps to `$B800` [E14].

The documented 512 KiB RAM requires 128 physical 4 KiB pages, while an 8-bit page entry can name 256 pages. The video driver additionally maps page `$F8` to logical page `$B`, explicitly describing the target as `$F8xxx` [E1, E5, E12]. Therefore, the page number appears to select a wider physical address/decode space than installed main RAM (**inferred**); not every page value should be backed by RAM.

Detailed MMU edge cases belong in the dedicated memory/MMU investigation. In particular, this snapshot does not establish power-on task contents, MMU enable state beyond a firmware comment that disabled is expected, precedence between MMU mapping and fixed I/O/ROM decode, readback from write-only registers, write behavior at read-only offsets, or behavior for absent physical pages [E5].

## 6. Core I/O and optional peripherals

### 6.1 On-board or baseboard-described devices

- **6551-compatible ACIA:** firmware accesses `$EF84` data, `$EF85` status/reset, `$EF86` command, and `$EF87` control. Initialization writes `$00` to status, `$0B` to command, and `$1E` to control, described as 9600 baud, 8 data bits, no parity, one stop bit [E6]. Exact baud reference clock, receive/transmit timing tolerance, IRQ wiring, and mirroring are unknown.
- **RTC:** firmware base is `$EF90`; it combines pairs of low-nibble values into BCD-like bytes and uses offsets `$0D-$0F` as control strobes during writes [E7]. The register-level investigation must preserve the observed access sequence without guessing a chip model.
- **Expansion slots:** the board names slot connectors J2/J4/J6/J8/J10/J14 and corresponding IRQ-assignment jumpers J1/J2/J5/J7/J9/J13 [E1]. The mapping from jumper position to CPU IRQ/NMI, interrupt polarity, sharing, priority, and acknowledge behavior is unknown.

### 6.2 Firmware-supported expansion devices

These devices are present in the banked DOS/65 driver dispatch table, but the baseboard note does not say they are fitted by default [E13]. Model them as optional/configurable peripherals.

| Device | Exercised address/decode | Source-observed role | Evidence |
|---|---|---|---|
| Dual ESP I/O card | `$E100` data 0, `$E101` data 1, `$E102` status | ANSI video, PS/2 keyboard, two serial channels, and network-console protocol through two ESP endpoints | [E10] |
| CH375/376 USB storage interface | `$E260` data, `$E261` command | USB mass-storage sector operations | [E9] |
| XT-IDE ISA card | `$E300-$E30E`, even-spaced ATA registers plus separate low/high data bytes | ATA/IDE identification and sector I/O | [E8] |
| ISA Multi-I/O card | Base `$E3E0`; LPT `$E3F0-$E3F2`; keyboard data `$E3FE`, status/command `$E3FF` | PC-style parallel printer and PS/2 keyboard controller | [E11] |
| Memory-mapped video card | 32 KiB device at physical page `$F8` and above, mapped by firmware into logical `$B000` | Text/color and graphics memory with mode/control registers | [E12] |

The video source calls the device a 32 KiB area but documents VRAM offsets extending through `$BFFF` for double-hires modes [E12]. That is internally inconsistent if all listed ranges are simultaneously resident in a single 32 KiB aperture. Treat video capacity, aliasing, and mode-dependent banking as unresolved, outside the minimum baseboard emulator.

## 7. Clock, bus, interrupt, and reset constraints

### 7.1 Clocks

The CPU clock frequency is **unknown** [A1]. The UART control byte selects an internal baud-rate divisor and firmware labels the result 9600 baud, but the oscillator/reference frequency is not documented [E6]. The RTC timebase and any expansion-bus clock are also **unknown** [A1]. Initial emulation should therefore use configurable CPU frequency and time-derived peripheral scheduling rather than encoding a purported board frequency.

### 7.2 Bus behavior

The only bus-level statement is “6 ISAish slots,” plus the CPU-visible ISA I/O allocation [E1, E2]. Data width, address-line availability, clock, wait states, read/write strobe timing, reset timing, DMA, terminal-count behavior, bus mastering, and open-bus value are **unknown** [A1]. `$EFE6` proves that firmware expected some observable “ISA TC” state, but does not define its lifecycle [E5].

### 7.3 Interrupts

The hardware notes prove configurable slot IRQ assignment but not the resulting signal topology [E1]. The ROM installs indirect RAM vectors for IRQ and NMI, and its hardware vectors target separate NMI/reset/IRQ entry points [E3]. Device-level IRQ enable/polarity and whether the ACIA, RTC, or slots are actually connected to IRQ or NMI remain unknown. Polling behavior in firmware must not be taken as proof that interrupts are physically absent [E6-E11].

### 7.4 Reset and default mapping

The ROM reset entry disables interrupts and decimal mode, initializes the stack, applies a delay, selects a console, installs software vectors, initializes paging and serial, prints a banner, and enters the monitor with `BRK` unless the video build takes its boot path [E3]. This proves required software-visible behavior after vector fetch, not the electrical reset duration or all power-on register values.

For a first emulator milestone, use ROM at `$F000-$FFFF`, I/O at `$E000-$EFFF`, MMU disabled for the initial vector fetch, and otherwise direct RAM below I/O (**inferred from firmware sequencing**) [E2, E3, E5]. MMU-disabled behavior and fixed-decode precedence require later hardware confirmation.

## 8. ROM and software stack inventory

### 8.1 ROM sources and variants

`6502PCbios.asm` is the ROM root. It includes common definitions, DOS/65 symbols, the monitor, serial, IDE, and pager sources; ESP support is conditional [E3]. The build file assembles base, `ESP`, and `VIDEO` variants, then creates `rom.hex` from the base and video outputs; the ESP half is present but commented out [E15]. Generated maps place all variants at CPU `$F000` and retain vectors at `$FFFA-$FFFF` [E17].

K1's `$6000-$6FFF` versus `$7000-$7FFF` labels refer to locations in the 8 KiB programmed ROM image, while either selected bank appears at CPU `$F000-$FFFF` (**inferred from the jumper note, linker placement, and Intel HEX layout**) [E1, E17, E18]. Exact image checksums, vector decoding, and bank contents belong in the ROM/reset investigation.

### 8.2 DOS/65 integration

The PC6502-specific DOS/65 target is assembled with `DOSBEGIN=47104` (`$B800`) and `PC6502` defined [E19]. The banked driver is linked at `$C000`; its dispatch table exposes console, storage, RTC, video, keyboard, and printer functions [E13, E16]. The loader stages the OS and driver into task 0 and task 1 respectively [E14]. These facts establish the software/MMU contract but do not specify disk geometry or API error behavior; those belong in the DOS/65 expectations investigation.

### 8.3 Evidence-bearing repository areas

| Area | Contents and evidentiary use |
|---|---|
| `documentation/PC6502_system_documentation.md` | Only PC6502-specific board overview in the snapshot: component summary, jumpers/connectors, and default address map [E1, E2]. |
| `PC6502_firmware_source/*.asm` | ROM, MMU, UART, storage, RTC, optional I/O, video, DOS driver, loader, and diagnostic source [E3-E15]. |
| `PC6502_firmware_source/*.lst` | Generated ca65 listings with emitted bytes and resolved addresses; the listings identify ca65 V2.18 / Ubuntu package 2.19-1 [E22]. |
| `PC6502_firmware_source/*.map` | Generated linker segment placement for ROM variants, driver, loader, and Multi-I/O test [E17]. |
| `PC6502_firmware_source/rom.hex` | Supplied 8 KiB Intel HEX programming image covering record addresses `$6000-$7FFF` [E18]. |
| `DOS65_OS/dos65_os/` | DOS/65 kernel sources, PC6502 conditional build, generated listings, and linker layouts [E19]. |
| `DOS65_OS/dos65_utilities/` and `DOS65_OS/software/` | Utilities and applications that can reveal higher-level I/O and memory assumptions; they are secondary evidence for hardware behavior [A1]. |
| `documentation/*.pdf` | Ten DOS/65 and language/tool manuals. They are operating-system references, not PC6502 schematics [A1]. |

The local Git metadata names `https://github.com/danwerner21/PC6502Emulator.git` as origin [E21]. The board and DOS/65 notes separately point to `https://github.com/danwerner21/6x0x-DOS65` for ROM/OS material [E1, E20]. Source comments credit Dan Werner for the original 2014 BIOS, a 2023 cleanup, the 2025 PC6502 port, and the 2025 banked driver/loader [E3, E13, E14]. This snapshot does not provide a usable commit identifier in its materialized branch refs [A1], so conclusions are pinned to paths and file contents rather than a commit hash.

## 9. Source and documentation conflicts

| Conflict | Resolution for emulator work |
|---|---|
| Board document title says `6809PC`; body and firmware say 6502PC/PC6502 [E1, E3]. | Treat title as stale; emulate a 6502 target. Keep exact CPU subtype unknown. |
| RAM is printed as `$0000-$E000`, overlapping I/O beginning at `$E000` [E2]. | Normalize default RAM to `$0000-$DFFF` and give I/O decode precedence at `$E000` (**inferred**). |
| Board says 512 KiB RAM, but MMU entries can select pages such as `$F8` [E1, E5, E12]. | Separate physical decode space from installed RAM. Allow device mappings and unmapped pages beyond RAM. |
| ACIA allocation is 16 bytes, while firmware exercises only `$EF84-$EF87` [E2, E6]. | Implement the four used addresses first; make wider mirroring a later evidence-based choice. |
| Video is described as 32 KiB, while documented mode ranges extend beyond one 32 KiB span [E12]. | Treat advanced video memory layout as unresolved and optional. |
| MMU comments use `$xFE0` notation and mention a “current IO page,” while current PC6502 firmware hardcodes the `$EFxx` page [E4, E5]. | Implement `$EFxx` as the confirmed software contract; do not invent relocatable I/O-page behavior until the MMU investigation resolves `$EFE7`. |
| ROM has a 4 KiB CPU window but `rom.hex` is 8 KiB [E1, E2, E18]. | Model a selectable 4 KiB bank at `$F000`; retain both supplied images as ROM variants. |
| Firmware source includes `../dos65_os/`, `../supermon/`, and outputs to `../bin/6502PC`, but those paths do not exist relative to `PC6502_firmware_source/` in this repository layout [E3, E15, A1]. | Treat checked-in listings/maps/HEX as historical build evidence. A reproducible build requires path/layout repair, which is outside this investigation. |

## 10. Explicit unknowns and validation priorities

### Priority 0 — required for cycle-credible baseboard emulation

1. Exact CPU part and supported instruction/timing behavior [A1].
2. CPU oscillator frequency and reset timing [A1].
3. MMU power-on state, complete register semantics, physical address width, fixed I/O/ROM precedence, and absent-page behavior [E5, A1].
4. ROM bank-select electrical behavior and which supplied half is the physical default [E1, E15, E18].
5. ACIA reference clock, full address decode/mirroring, IRQ connection, and hardware reset defaults [E2, E6, A1].
6. RTC chip model, oscillator/timebase, register electrical semantics, and interrupt behavior [E1, E7, A1].
7. Open-bus and unmapped-address read values [E2, A1].

### Priority 1 — required for expansion compatibility

1. Slot connector pinout, voltage levels, clocking, wait states, and exact relationship to IBM ISA [E1, A1].
2. IRQ jumper position mapping, polarity, sharing, and priority [E1, A1].
3. `$EFE6` terminal-count source and clear behavior [E5].
4. Optional-card presence/default configuration and address-decode mirrors [E8-E13, A1].
5. Video-card physical capacity, mode-dependent banking, and mapping conflicts [E12].
6. ATX power-control, reset-switch, battery, console-level, and CTS-jumper electrical details [E1, A1].

## 11. Minimum emulator-facing contract

Until higher-confidence evidence is available, an emulator can safely target this functional envelope:

1. Provide a configurable 6502-compatible CPU core and do not claim a specific silicon subtype [E3, E19].
2. Present a 64 KiB logical address space with initial RAM below `$E000`, I/O at `$E000-$EFFF`, and a selected 4 KiB ROM bank at `$F000-$FFFF` (**partly inferred**) [E2, E3].
3. Provide 512 KiB installed main RAM while keeping the MMU's wider page-selection/decode space distinct from RAM capacity [E1, E5, E12].
4. Implement the MMU task/edit/control addresses exercised by firmware and reproduce task 0/task 1 initialization [E4, E5].
5. Implement 6551-compatible polling at `$EF84-$EF87` and RTC accesses at `$EF90-$EF9F`; keep timing and interrupt details configurable [E6, E7].
6. Treat slot devices, storage cards, ESP, Multi-I/O, and video as optional modules rather than mandatory baseboard devices [E1, E8-E13].
7. Boot from the ROM reset vector and support the banked `$C000` driver path needed by the DOS/65 loader [E3, E5, E13, E14].
8. Preserve unknown and open regions explicitly in the implementation; do not silently convert every MMU page or I/O hole into zero-filled RAM [E2, E5].

## 12. Evidence index

- **[E1]** `documentation/PC6502_system_documentation.md:1-24` — board identity, component summary, image link, jumpers, power/reset, battery, and console connectors.
- **[E2]** `documentation/PC6502_system_documentation.md:28-41` — default memory and I/O maps.
- **[E3]** `PC6502_firmware_source/6502PCbios.asm:2-18,26-88,215-228` — BIOS provenance, initialization flow, ROM origin, service stubs, and vectors.
- **[E4]** `PC6502_firmware_source/bios_defines.asm:6-15` — PC6502 I/O, shadow-ROM, driver-dispatch, and MMU constants.
- **[E5]** `PC6502_firmware_source/bios_pager.asm:8-58,74-104` — MMU contexts, register descriptions, initialization, page switching, and banked far calls.
- **[E6]** `PC6502_firmware_source/bios_serial.asm:14-23,25-82,86-157` — ACIA addresses, register bits, initialization, and polling behavior.
- **[E7]** `PC6502_firmware_source/bios_rtc.asm:3-94,95-180` — RTC base, nibble access, control sequence, unsupported auxiliary features, and displayed fields.
- **[E8]** `PC6502_firmware_source/bios_ide.asm:1-35,137-202,282-482` — XT-IDE identity, register map, probe, and sector transfers.
- **[E9]** `PC6502_firmware_source/bios_ch375.asm:1-46,49-157` — CH375/376 identity, two-register interface, commands, and initialization.
- **[E10]** `PC6502_firmware_source/bios_esp.asm:1-44,56-209,213-385` — dual ESP identity, address map, services, probe, and protocol operations.
- **[E11]** `PC6502_firmware_source/bios_multi.asm:1-51,79-180` — ISA Multi-I/O identity, LPT/keyboard addresses, register bits, and probe/init behavior.
- **[E12]** `PC6502_firmware_source/bios_video.asm:1-62,22-51,64-131` — video-card size claim, map, MMU page selection, and initialization.
- **[E13]** `PC6502_firmware_source/dos65drv.asm:1-13,15-137` — banked driver provenance, link address, dispatch functions, and included device drivers.
- **[E14]** `PC6502_firmware_source/loader.asm:1-86` — PC6502 loader provenance, task selection, copies, and OS entry.
- **[E15]** `PC6502_firmware_source/Makefile:1-54` — ROM variants, generated artifacts, HEX composition, loader composition, and expected output layout.
- **[E16]** `PC6502_firmware_source/dos65.cfg:1-17` — linker memory and segment origins.
- **[E17]** `PC6502_firmware_source/6502PCbios.map:1-29`, `PC6502_firmware_source/6502PCbiosesp.map:1-29`, `PC6502_firmware_source/6502PCbiosvideo.map:1-29`, `PC6502_firmware_source/dos65drv.map:1-23`, and `PC6502_firmware_source/loader.map:1-23` — generated segment placement and sizes.
- **[E18]** `PC6502_firmware_source/rom.hex:1-257` — supplied Intel HEX data records at `$6000-$7FFF` and EOF record.
- **[E19]** `DOS65_OS/dos65_os/Makefile:1-28` — PC6502-specific DOS/65 build at `$B800` and generated S-record target.
- **[E20]** `documentation/DOS65_Description.md:1-23` — DOS/65 attribution, supported-system overview, ROMWBW mapping note, and upstream repository reference.
- **[E21]** `.git/config:1-11` — local snapshot's configured origin URL and main branch name.
- **[E22]** `PC6502_firmware_source/6502PCbios.lst:1-14`, `PC6502_firmware_source/dos65drv.lst:1-14`, and `PC6502_firmware_source/loader.lst:1-14` — assembler version, source roots, and generated listing provenance.
- **[A1]** Recursive inventory of `/mnt/fileserver/Vintage/Projects/PC6502Emulator` on 2026-06-27. Relevant present roots are `DOS65_OS/`, `PC6502_firmware_source/`, `documentation/`, and `specifications/`. The PC6502 image referenced at `documentation/images/6502PC.jpg`, current-layout include targets `dos65_os/` and `supermon/`, output tree `bin/6502PC/`, and `docs/investigation/` were absent at investigation time.
