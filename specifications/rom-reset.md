# PC6502 ROM Images and Reset Contract

**Investigation date:** 2026-06-27  
**Scope:** ROM and reset evidence in the local `PC6502Emulator` repository snapshot  
**Target audience:** Emulator implementers

## 1. Result summary

The supplied `PC6502_firmware_source/rom.hex` is an 8 KiB Intel HEX programming image containing two independently selectable 4 KiB ROM banks. It is not an 8 KiB ROM mapped at CPU address `$6000`. Hardware jumper K1 selects either image-file region `$6000-$6FFF` or `$7000-$7FFF`; the selected 4 KiB bank appears to the 6502 at CPU `$F000-$FFFF` [E1, E2, E5].

| Programming-image region | Build represented | CPU-visible window | Reset target | Post-initialization behavior |
|---|---|---|---|---|
| `$6000-$6FFF` | Base/serial monitor build | `$F000-$FFFF` | `$F000` | Executes `BRK` and enters the monitor |
| `$7000-$7FFF` | `VIDEO` build | `$F000-$FFFF` | `$F000` | Jumps directly to the IDE boot routine |

An emulator must choose one 4 KiB bank and map it at `$F000-$FFFF` before the CPU fetches `$FFFC/$FFFD`. It must not map the Intel HEX record addresses directly into the 6502 address space. The base bank is the safer initial emulator default because it reaches an interactive monitor rather than requiring a working XT-IDE device, but the physical board's power-on K1 position is unknown.

## 2. Confidence terminology

- **Confirmed** — directly stated by local board documentation and/or represented by generated machine-code bytes.
- **Inferred** — the interpretation that consistently explains multiple local sources, but is not backed by schematics or hardware measurements.
- **Unknown** — not established by this repository snapshot.

## 3. Image inventory and provenance

### 3.1 Supplied artifact

| Property | Value | Status |
|---|---|---|
| Path | `PC6502_firmware_source/rom.hex` | Confirmed |
| Encoding | Intel Hexadecimal (MCS-86), 16-bit record addresses | Confirmed by `srec_info` |
| Record-address span | `$6000-$7FFF`, contiguous | Confirmed by `srec_info` and records 1-256 [E2] |
| Decoded size | 8192 bytes (`$2000`) | Confirmed |
| Text-file size | 19,468 bytes | Confirmed in this snapshot |
| Whole-file SHA-256 | `0bc03df6513339bae79cb252f15c1d0dff7424d1906cca9915852c9ea42ff08f` | Confirmed in this snapshot |
| `$6000-$6FFF` decoded-bank SHA-256 | `ff654b143d99f387d23e7998082bb4bbc15ff3f6ea223f2f607e3790283e6494` | Confirmed in this snapshot |
| `$7000-$7FFF` decoded-bank SHA-256 | `409e1b92bf452b6b6fcf96c9b35c48d0e1345b0cb26f9df55421a8f352daa1e7` | Confirmed in this snapshot |

The source header identifies the BIOS as Dan Werner's code, originally written 2014-01-01, cleaned up 2023-01-22, and ported to PC6502 on 2025-12-06 [E3]. No version string, source commit, build timestamp, or artifact checksum is embedded in `rom.hex`; provenance beyond the checked-in source, listings, maps, and build rules is unknown.

### 3.2 How the two banks are produced

The build assembles the same `6502PCbios.asm` root three ways: base, `ESP`, and `VIDEO`. It links every variant at CPU `$F000` with vectors at `$FFFA-$FFFF` [E4, E6]. The `rom.hex` rule then:

1. Extracts the base output's ROM bytes and relocates them to Intel HEX `$6000-$6FFF`.
2. Extracts the `VIDEO` output and relocates it to `$7000-$7FFF`.
3. Concatenates those halves into `rom.hex` [E5].

The apparent extraction offset includes the linker output's `$0100` file-origin bias; it does not change the firmware's CPU link address. The generated maps independently confirm that both banks' `TROM`, `IVECTOR`, and `VECTORS` segments occupy CPU `$F000-$FFFF` [E6].

The `ESP` variant has a checked-in listing and map, but the line that would place it in the second ROM half is commented out and replaced by the `VIDEO` build [E5]. Therefore:

- `rom.hex` contains **base + VIDEO**.
- It does **not** contain the ESP variant.
- There is no standalone ESP binary or HEX image in this snapshot.

The current artifact inventory is consequently:

| Name | Present? | Role/provenance |
|---|---|---|
| `rom.hex` | Yes | Final base + VIDEO programming image |
| `6502PCbios.lst` / `.map` | Yes | Generated address/byte evidence for the base bank |
| `6502PCbiosvideo.lst` / `.map` | Yes | Generated address/byte evidence for the VIDEO bank |
| `6502PCbiosesp.lst` / `.map` | Yes | Generated evidence for the unpackaged ESP variant |
| `6502PCbios.out` | No | Historical base linker output named by the Makefile |
| `6502PCbiosvideo.out` | No | Historical VIDEO linker output named by the Makefile |
| `6502PCbiosesp.out` | No | Historical ESP linker output named by the Makefile |
| `rom.h1` / `rom.h2` | No | Temporary HEX halves deleted by the build rule |

`pcdos65.s19` is a separately generated disk/loader payload, not a ROM image. It is also absent from this snapshot; only its source and build recipe remain [E5, E9].

## 4. Image address translation

Use the following translation when loading either bank:

```text
cpu_address = $F000 + (image_address - selected_bank_start)
```

Examples:

| Selected image half | Image address | CPU address | Meaning |
|---|---:|---:|---|
| Base | `$6000` | `$F000` | Reset entry code |
| Base | `$6FFA` | `$FFFA` | NMI vector low byte |
| Base | `$6FFF` | `$FFFF` | IRQ vector high byte |
| VIDEO | `$7000` | `$F000` | Reset entry code |
| VIDEO | `$7FFA` | `$FFFA` | NMI vector low byte |
| VIDEO | `$7FFF` | `$FFFF` | IRQ vector high byte |

The K1 documentation explicitly labels pins 1-2 as selecting image `$6000-$6FFF` and pins 3-4 as selecting `$7000-$7FFF` [E1]. No firmware write changes this selection. Model it as a machine configuration or virtual jumper sampled at reset, not as a memory-mapped bank register.

## 5. Exact hardware vectors

6502 vectors are little-endian words: low byte at the lower address, then high byte. The bytes below are present both in `rom.hex` and in the generated listings [E2, E7].

### 5.1 Base/serial monitor bank

| CPU addresses | Image addresses | Exact bytes | Decoded target | Target role |
|---|---|---|---:|---|
| `$FFFA-$FFFB` | `$6FFA-$6FFB` | `59 F0` | `$F059` | `NINTERRUPT`, indirect NMI dispatcher |
| `$FFFC-$FFFD` | `$6FFC-$6FFD` | `00 F0` | `$F000` | `COLD_START` |
| `$FFFE-$FFFF` | `$6FFE-$6FFF` | `43 F0` | `$F043` | `INTERRUPT`, BRK/IRQ discriminator |

Reproduction: `59 F0` decodes as `$F0 << 8 | $59 = $F059`; the same rule gives `$F000` and `$F043`.

### 5.2 VIDEO/automatic-boot bank

| CPU addresses | Image addresses | Exact bytes | Decoded target | Target role |
|---|---|---|---:|---|
| `$FFFA-$FFFB` | `$7FFA-$7FFB` | `60 F0` | `$F060` | `NINTERRUPT`, indirect NMI dispatcher |
| `$FFFC-$FFFD` | `$7FFC-$7FFD` | `00 F0` | `$F000` | `COLD_START` |
| `$FFFE-$FFFF` | `$7FFE-$7FFF` | `4A F0` | `$F04A` | `INTERRUPT`, BRK/IRQ discriminator |

The target differences are expected: the VIDEO conditional adds four bytes near the start and replaces the base bank's one-byte `BRK` path with a three-byte absolute `JMP`, shifting later labels by seven bytes [E7]. Both reset vectors still point to `$F000`.

### 5.3 ROM service jump table

The nine bytes at CPU `$FFF0-$FFF8` are a public-looking absolute-jump table; `$FFF9` is an unused zero byte before the hardware vectors [E3, E7]. These targets are variant-specific and callers must enter through the fixed stub addresses rather than hard-code the internal destinations.

| Stub | Base bytes/target | VIDEO bytes/target | Service |
|---:|---|---|---|
| `$FFF0` | `4C 43 FD` → `$FD43` | `4C 4A FD` → `$FD4A` | Banked far-call wrapper |
| `$FFF3` | `4C EE F7` → `$F7EE` | `4C F5 F7` → `$F7F5` | S-record loader |
| `$FFF6` | `4C 24 FD` → `$FD24` | `4C 2B FD` → `$FD2B` | MMU page setup |

## 6. Reset execution flow

### 6.1 Common path from `$F000`

Both supplied banks execute the following sequence [E3, E7]:

1. `SEI`, `CLD`, load `X=$FF`, and `TXS`. This disables maskable interrupts, clears decimal mode, and establishes stack pointer `$FF`.
2. Execute a nested X/Y decrement loop as an uncalibrated startup delay. Its elapsed real time depends on the unknown CPU clock.
3. Store console selector `$04` (serial) at zero-page `CONSOLE` (`$3A`). The VIDEO build immediately overwrites it with decimal 19 (`$13`).
4. Initialize RAM indirect vectors `$0035-$0036` (IRQ) and `$0037-$0038` (NMI) to the build's `IRQROUTINE` (`$F041` base or `$F048` VIDEO).
5. Call `INITPAGES`, which initializes task 1, task 0, selects task 0, and enables the MMU.
6. Call `SERIALINIT` in both variants.
7. Select MMU task 1, print the startup banner, and return to task 0.
8. Clear input-buffer byte `$0300`.
9. Diverge by variant as described below.

Although the VIDEO build stores console selector `$13`, the ROM root still includes the serial implementation, always calls `SERIALINIT`, and its non-ESP output routine jumps to `WRSER1` [E3]. The name `VIDEO` must not be interpreted as evidence that serial hardware is optional during this ROM's startup. What selector `$13` affects outside the visible ROM root is not established.

### 6.2 Base bank: monitor entry

At `$F040`, the base bank executes `BRK` [E7]. The CPU pushes the return state and fetches the IRQ/BRK vector `$F043`. `INTERRUPT` tests the stacked B flag; for `BRK`, it preserves A/X/Y and jumps into the included Supermon `BRKROUTINE`. For a hardware IRQ, it jumps indirectly through RAM vector `$0035/$0036`. NMI enters at `$F059` and jumps indirectly through `$0037/$0038` [E3].

The monitor provides a `BOOT` command through the included IDE code, so the base bank can boot DOS/65 after interactive monitor entry. Exact monitor command UX is outside this ROM/reset contract.

### 6.3 VIDEO bank: automatic IDE boot

At `$F044`, the VIDEO bank jumps to `BOOT` at `$FBA1` [E7]. The ROM then [E8]:

1. Initializes XT-IDE.
2. Selects drive 0 and LBA 0.
3. Reads one 512-byte sector at a time into staging buffer `$0400-$05FF`.
4. Copies each sector into RAM beginning at `$0800`.
5. Increments LBA and destination by 512 bytes until the destination reaches `$8000`.
6. Jumps to `$0800`.

This is exactly 60 sectors (`($8000-$0800)/512 = 60`), LBAs 0-59, totaling 30,720 bytes. A read failure branches to the ROM monitor's `ERROR` path rather than jumping to partially loaded RAM.

The expected program at `$0800` is the PC6502 DOS/65 loader [E9]. It keeps MMU task 0 active while copying staged DOS/65 data from `$1000` to logical `$B800-$DFFF`; switches to task 1 while copying banked drivers from `$4000` to logical `$C000-$DFFF` (physical pages `$10-$11`); returns to task 0; and jumps to DOS/65 at `$B800`.

```text
RESET vector $F000
  -> common ROM/MMU/serial initialization
  -> base bank: BRK -> monitor -> optional BOOT
  -> VIDEO bank: BOOT immediately
       -> XT-IDE LBA 0..59 -> RAM $0800..$7FFF
       -> JMP $0800
       -> DOS/65 loader stages OS and banked drivers
       -> JMP $B800
```

## 7. ROM, RAM, and MMU assumptions

### 7.1 Confirmed or required by executed firmware

- The selected ROM bank is CPU-visible at `$F000-$FFFF` for the initial vector fetch and subsequent instruction reads [E1, E3, E6].
- I/O is visible in the `$E000-$EFFF` window, including MMU registers at `$EFD0-$EFE2` and serial/IDE devices used during startup [E1, E10].
- The BIOS expects writable zero page, stack page, `$0300`, IDE staging RAM `$0400-$05FF`, and boot destination RAM `$0800-$7FFF`.
- The BIOS expects the MMU disabled on entry; its first paging action explicitly disables it and comments that it "should be already" disabled [E10].
- Task 1 is programmed one-to-one except CPU pages `$C` and `$D` map to physical pages `$10` and `$11`. Task 0 is programmed one-to-one. The BIOS then selects task 0 and enables the MMU [E10].
- ROM code remains executable after the BIOS enables the MMU with task 0's identity mapping and while task 1 is active with only pages C/D changed. Any emulator model must preserve that observed case.

### 7.2 Emulator-facing inferences

- Reset the emulated MMU to disabled before vector fetch. Resetting active/setup task selectors to 0 is a reasonable deterministic choice, but firmware does not depend on those initial selector values because it rewrites them.
- Give fixed I/O and selected-ROM decode precedence sufficient to reproduce the documented reset path. The repository does not prove the complete priority rules for arbitrary MMU mappings.
- Treat ROM writes as ignored or otherwise non-mutating. This follows the documented ROM designation, but no local electrical evidence defines bus/open-bus details on such writes.
- Keep ROM selection outside guest software control unless later hardware evidence identifies a register. K1 is the only documented selector.

### 7.3 Unknown behavior that must remain explicit

- Whether an MMU mapping of logical page `$F` to a non-`$0F` physical page can hide the ROM.
- Whether ROM and I/O always override MMU translation, or only do so in specific task/page states.
- Power-on contents of all 64 MMU task maps and all MMU registers other than the firmware expectation that enable is clear.
- Which K1 bank was normally shipped or selected by default.
- ROM device type, access time, electrical write response, and bank-select timing.
- CPU subtype, CPU frequency, reset-cycle duration, and the real-time duration intended by the startup delay.
- Whether switching K1 while running is electrically safe or immediately changes the visible bank. An emulator should apply a bank change only on reset unless a hardware test establishes hot-switch behavior.

## 8. Variants and contradictions

| Observation | Resolution |
|---|---|
| Board notes say the machine has a 4 KiB ROM, while `rom.hex` decodes to 8 KiB. | The 8 KiB device holds two K1-selectable 4 KiB images; only one is CPU-visible at a time [E1, E2]. |
| `rom.hex` uses `$6000-$7FFF`, while firmware is linked at `$F000`. | `$6000/$7000` are programmer-image bank locations; both selected halves map to CPU `$F000` [E1, E5, E6]. |
| The board document is titled `6809PC`. | Its body, vectors, source, and listings are 6502-specific; treat the title as a stale copy/paste error [E1, E3]. |
| Board RAM is written as `$0000-$E000`, overlapping I/O beginning at `$E000`. | Use RAM `$0000-$DFFF` in the default decode and give I/O precedence at `$E000` [E1]. |
| A generated ESP map/listing exists, but K1's second half contains VIDEO. | Current build rule comments out ESP packaging and explicitly packages VIDEO [E5]. |
| `VIDEO` suggests a non-serial startup, but the ROM calls serial init and outputs through serial. | Emulate the emitted code. Do not remove the UART merely because this bank auto-boots or writes console selector `$13` [E3, E7]. |

## 9. Minimum implementation contract

An emulator ROM/reset implementation is conformant to the evidence in this snapshot when it:

1. Parses `rom.hex` as an 8 KiB programming image or consumes equivalent extracted 4 KiB bank blobs.
2. Exposes a configuration selecting base (`$6000-$6FFF`) or VIDEO (`$7000-$7FFF`).
3. Maps only the selected bank to CPU `$F000-$FFFF` before reset vector fetch.
4. Returns the exact vector bytes documented in section 5 and begins execution at `$F000`.
5. Starts with MMU translation disabled, writable low RAM, and I/O/ROM decode available.
6. Preserves ROM visibility through the BIOS's observed task-0/task-1 mappings.
7. Supports the serial and MMU writes made by the common reset path.
8. For the base bank, correctly delivers `BRK` through `$FFFE/$FFFF` so the monitor is reached.
9. For the VIDEO bank, supplies XT-IDE behavior capable of loading 60 sectors to `$0800-$7FFF`, or exposes the ROM's error path when no device/image is configured.
10. Keeps unproven behavior—hot bank switching, ROM/MMU priority for arbitrary page-F maps, exact CPU subtype/timing, and hardware default K1 position—configurable or documented as unsupported.

## 10. Reproduction commands

The following read-only commands reproduce the artifact facts and vector decoding used above:

```bash
sha256sum PC6502_firmware_source/rom.hex
srec_info PC6502_firmware_source/rom.hex -Intel

srec_cat PC6502_firmware_source/rom.hex -Intel \
  -crop 0x6000 0x7000 -offset -0x6000 -o - -Binary | sha256sum
srec_cat PC6502_firmware_source/rom.hex -Intel \
  -crop 0x7000 0x8000 -offset -0x7000 -o - -Binary | sha256sum

srec_cat PC6502_firmware_source/rom.hex -Intel \
  -crop 0x6FE0 0x7000 -o - -HEX_Dump
srec_cat PC6502_firmware_source/rom.hex -Intel \
  -crop 0x7FE0 0x8000 -o - -HEX_Dump
```

`srec_info` also validates the Intel HEX records sufficiently to parse the complete `$6000-$7FFF` range. The checked-in generated listings provide an independent address/byte cross-check.

## 11. Evidence index

- **[E1]** `documentation/PC6502_system_documentation.md:1-41` — 4 KiB ROM statement, K1 image-bank selection, CPU-visible memory map, and I/O/MMU windows.
- **[E2]** `PC6502_firmware_source/rom.hex:1-257` — complete supplied Intel HEX image; record 128 contains the base service stubs/vectors, record 129 starts the VIDEO half, and record 256 contains the VIDEO stubs/vectors.
- **[E3]** `PC6502_firmware_source/6502PCbios.asm:2-18,26-177,215-230` — authorship/port dates, ROM origin, common reset logic, conditional behavior, interrupt dispatch, I/O paths, and vector declarations.
- **[E4]** `PC6502_firmware_source/dos65.cfg:1-17` — linker file origin and CPU-address segment placement.
- **[E5]** `PC6502_firmware_source/Makefile:1-10,26-33` — base/ESP/VIDEO builds and composition of `rom.hex` from base plus VIDEO.
- **[E6]** `PC6502_firmware_source/6502PCbios.map:1-27`, `PC6502_firmware_source/6502PCbiosvideo.map:1-27`, and `PC6502_firmware_source/6502PCbiosesp.map:1-27` — linked CPU ranges and variant sizes.
- **[E7]** `PC6502_firmware_source/6502PCbios.lst:195-285,3064-3078` and `PC6502_firmware_source/6502PCbiosvideo.lst:195-290,3064-3078` — exact reset bytes, branch difference, service stubs, and vector bytes/targets.
- **[E8]** `PC6502_firmware_source/bios_ide.asm:624-669` — ROM IDE boot loop, staging/destination pointers, sector iteration, handoff, and failure path.
- **[E9]** `PC6502_firmware_source/loader.asm:1-86` and `PC6502_firmware_source/Makefile:42-54` — staged DOS/65 image construction and loader handoff through MMU tasks to `$B800`.
- **[E10]** `PC6502_firmware_source/bios_defines.asm:6-15,24-32` and `PC6502_firmware_source/bios_pager.asm:8-70` — I/O/ROM constants, input buffer, expected MMU reset state, task maps, activation, and enable sequence.
