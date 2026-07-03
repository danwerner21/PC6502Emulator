---
schema: gc.build.requirements.v1
workflow:
  id: pe-w1b
  formula: build-from-requirements
methodology:
  pack: gascity
  name: build-from-requirements
producer:
  formula: build-from-requirements
  stage: requirements
  attempt: 1
status: approved
trace:
  upstream:
    - path: beads/pe-v4x
      hash: bead:pe-v4x
      ids:
        - REQ-M1
        - REQ-M2
        - REQ-M3
        - REQ-M4
        - REQ-M5
        - REQ-M6
    - path: specifications/system-reference.md
      hash: "sha256:463cb0de339d9dfb4544443aa597f6079f6d124936fd5dfaccf62513e29694b8"
    - path: specifications/hardware-spec.md
      hash: "sha256:8e032009fcfb4cdfdf321439e11c940785f03109d2e620208d47e8284f7cc375"
    - path: specifications/memory-mmu.md
      hash: "sha256:34ad65565a09b0e50dc114c4254e5d075130183f1939d802e40fce277764a054"
    - path: specifications/rom-reset.md
      hash: "sha256:1626177e428179b513a3b6b7f43c11f7419d908f96073b8e536a816a3697e60b"
    - path: specifications/dos65-expectations.md
      hash: "sha256:03b3451556eee3aec36280173a4b81701be3ee9a19743d66a1039c00fa7d3e34"
  coverage:
    - id: REQ-M1
      status: covered
    - id: REQ-M2
      status: covered
    - id: REQ-M3
      status: covered
    - id: REQ-M4
      status: covered
    - id: REQ-M5
      status: covered
    - id: REQ-M6
      status: covered
---

# Requirements: PC6502 Emulator — Development Milestones

## Problem Statement

The PC6502 is a custom 6502-based single-board computer with a 64-task MMU, 512 KiB SRAM, dual ROM banks, a 6551-compatible UART, battery-backed RTC, and XT-IDE storage. Five hardware investigations have been synthesized into authoritative specification documents (`system-reference.md`, `hardware-spec.md`, `memory-mmu.md`, `rom-reset.md`, `dos65-expectations.md`). No emulator source exists and no physical board is available for development. Correctness must be verified by running actual `rom.hex` and DOS/65 disk images through the emulator.

## W6H

| Axis | Detail |
| --- | --- |
| **Who** | Firmware and OS developers who need to run BIOS and DOS/65 code without access to a physical PC6502 board |
| **What** | A software emulator for the PC6502 single-board computer: 6502 CPU, 64-task MMU, 6551 ACIA, XT-IDE controller, RTC-72421/72423, and safe stubs for absent optional cards |
| **Why** | No physical board is available; all ROM and OS code must be verified through software emulation before hardware is accessible; synthetic unit tests are insufficient — firmware behavior under its real boot path is the only acceptable gate |
| **When** | At each of six milestones, gated by serial output produced by the actual `rom.hex` and DOS/65 disk images; a milestone is not complete until the firmware itself produces the expected observable output |
| **Where** | A development host (Linux) with a serial console abstraction; headless command-line, no video output required for any milestone |
| **How** | Staged milestone development: M1 establishes the CPU and serial path; M2 adds the MMU; M3 adds XT-IDE and VIDEO ROM boot; M4 brings DOS/65 to a cold-boot prompt; M5 exercises disk read/write; M6 adds RTC and configuration hardening |

## User Stories

**US-1.** As a firmware developer without a physical PC6502, I want to run `rom.hex` through an emulator and reach the Supermon interactive prompt on a serial console, so that I can develop and debug BIOS code iteratively without physical hardware.

**US-2.** As a DOS/65 developer, I want to cold-boot DOS/65 from a disk image and see the `A>` prompt on the emulator's serial console, so that I can verify the operating system loads and initializes correctly end-to-end.

**US-3.** As a developer testing file I/O, I want to run `DIR A:` in DOS/65 and see directory entries from a CP/M disk image, so that I can validate sector addressing, the DOS/65 disk-I/O SIM interface, and the CP/M directory structure.

**US-4.** As a developer investigating hardware unknowns (CPU subtype, open-bus value, ROM bank, RTC policy, shadow-address strap), I want the emulator to expose each unknown as a named configuration option, so that I can test multiple hardware configurations before physical validation.

**US-5.** As a tester, I want each milestone to have a clear, observable serial-output gate, so that milestone completion is unambiguous and not reliant solely on internal unit tests or mock substitutes.

## Technical Stories

**TS-1.** The emulator must execute all documented 6502 opcodes with correct flag behavior, stack pointer, and interrupt dispatch (NMI/RESET/IRQ/BRK); NMOS vs. CMOS subtype must be a configuration option.

**TS-2.** The emulator must model a 20-bit physical address space: 512 KiB SRAM at physical `$00000–$7FFFF`, I/O overlay at CPU `$E000–$EFFF`, ROM bank at CPU `$F000–$FFFF`; MMU-disabled mode uses the identity mapping for the CPU-visible lower 64 KiB.

**TS-3.** The emulator must model the 64-task MMU: 1,024-byte map store, 16-entry edit window at `$EFD0–$EFDF`, control/status registers at `$EFE0–$EFE7`, enable latch, and task-mask write behavior; map contents at power-on are indeterminate.

**TS-4.** The emulator must model the 6551-compatible ACIA at `$EF84–$EF87`: TX polling (TDRE always ready), RX injection (RDRF set on injected byte), programmed reset (`$00` written to `$EF85`), command and control register writes.

**TS-5.** The emulator must model the XT-IDE at `$E300–$E30E`: BSY/DRQ/ERR status bits, sector read command (`$20`), SET FEATURES (`$EF`), IDENTIFY (`$EC`), and 512-byte sector transfer; probe writes to `$E300–$E330` must not crash or corrupt disk state.

**TS-6.** The emulator must model the RTC-72421/72423 at `$EF90–$EF9F`: low-nibble-only data registers, control-register write sequence, STOP/RESET bit behavior; RTC policy (host clock, persistent image, or fixed test epoch) must be configurable.

**TS-7.** The emulator must load `rom.hex` as an 8 KiB image, select the K1 bank (Base vs VIDEO) via configuration before the reset vector fetch, and serve raw disk image sectors for XT-IDE transfers.

**TS-8.** Absent optional peripherals (CH375 at `$E260–$E261`, dual ESP at `$E100–$E102`, Multi-I/O keyboard at `$E3FE–$E3FF`) must return a safe failure without crashing; unmapped I/O reads return the configured open-bus value; unmapped writes are silently discarded.

**TS-9.** All P0 unknowns from `system-reference.md §9` (R0.1–R0.7) must be exposed as named emulator configuration knobs rather than hidden defaults.

## Behavior Requirements

**BR-1: Reset vector.** After reset with the Base bank selected, reading `$FFFC–$FFFD` returns `$00 $F0`; with the VIDEO bank selected, the same addresses return `$00 $F0`. (Both banks share the same reset entry point.)

**BR-2: ACIA init sequence.** Writing `$00` to `$EF85`, `$0B` to `$EF86`, and `$1E` to `$EF87` in order must not crash the emulator; subsequent TDRE polling must exit without hanging.

**BR-3: MMU task mask.** Writing `$FF` to `$EFE0` and `$EFE1` selects active task `$3F`; reading `$EFE4` returns `$3F` in bits 5:0 and the enable status in bit 7.

**BR-4: MMU enable.** Writing `$01` to `$EFE2` activates translation on the next bus cycle; task-0 logical addresses then translate through the installed map, not through the identity map.

**BR-5: XT-IDE probe tolerance.** Writes of `$FF` and `$00` to `$E300–$E330` do not alter disk state and do not cause the emulator to crash or return an error status.

**BR-6: Open-bus reads.** Reads to `$EFA0–$EFCF`, `$EFF0–$EFFF`, and unassigned MMU offsets return the configured open-bus byte (never implicitly `$00`); the open-bus policy must be a named, runtime-settable configuration.

**BR-7: Absent-device safety.** Firmware accessing absent CH375, ESP, or Multi-I/O must not hang, crash, or corrupt emulator state; the DOS prompt must return normally after a failed device access.

**BR-8: ROM write protection.** A write to CPU `$F000–$FFFF` must not update hidden SRAM; the ROM contents visible on subsequent reads must be unchanged.

**BR-9: Physical address holes.** Physical pages above `$7F` that are not I/O or ROM are holes: reads return the open-bus value, writes are discarded, and no RAM is silently backing those addresses.

## Example Mapping

### M1 — CPU + ACIA: reset path to Supermon

| Rule | Given | When | Then |
| --- | --- | --- | --- |
| Reset vector decode | Base bank loaded, MMU disabled | CPU fetches `$FFFC–$FFFD` | Returns `$00 $F0`; PC = `$F000` |
| BIOS startup sequence | CPU at `$F000` | Executes SEI / CLD / LDX `$FF` / TXS | Stack pointer = `$FF`; no CPU fault |
| ACIA init | CPU at SERIALINIT | Writes `$00`→`$EF85`, `$0B`→`$EF86`, `$1E`→`$EF87` | ACIA does not fault; TDRE bit 4 set |
| Banner output | UART initialized | TDRE polling loop executes | Each banner character appears on serial console |
| BRK dispatch | End of Base reset path | BRK executes | `$FFFE–$FFFF` vector reached; Supermon prompt `>` on serial |
| Go command | Supermon prompt active | User sends `G F000` | Reset path re-executes; banner re-emitted without hang |

### M2 — MMU: INITPAGES, map round-trip, address translation

| Rule | Given | When | Then |
| --- | --- | --- | --- |
| INITPAGES completion | MMU registers reset state | BIOS INITPAGES runs | Reading `$EFE4` returns `$00` with bit 7 set (task 0, enabled) |
| Task-0 map round-trip | MMU edit window for task 0 | Writes 16 known bytes to `$EFD0–$EFDF` | Read back returns identical 16 bytes |
| Task-1 isolation | Task-1 map installed | CPU `$C000–$DFFF` accessed in task 1 | Physical address = `$10000–$11FFF` (not `$0C000–$0DFFF`) |
| Identity map | Task-0 MMU enabled | CPU reads `$1234` | Physical address = `$01234`; read from SRAM |
| Banner persists | MMU enabled, task 0 active | Normal code path continues | Serial banner still printed; no hang or page fault |
| Task mask | `$FF` written to `$EFE0` | Supermon reads `$EFE4` | Bits 5:0 = `$3F` |

### M3 — XT-IDE + VIDEO ROM: 60-sector boot load

| Rule | Given | When | Then |
| --- | --- | --- | --- |
| VIDEO reset vector | VIDEO bank selected | CPU fetches `$FFFC–$FFFD` | Returns `$00 $F0` |
| Probe write safety | XT-IDE model present | Writes `$FF`/`$00` to `$E300–$E330` | No crash, no disk corruption |
| SET FEATURES | VIDEO ROM executing | Writes `$EF` to command register | BSY clears; DRQ and ERR absent |
| 60-sector read | LBA 0 addressed | Sector read command issued 60 times | All transfers succeed; no serial error output |
| Loader entry | Sectors copied to `$0800` | Execution continues | PC = `$0800`; debugger or trace log confirms |

### M4 — DOS/65 cold boot to prompt

| Rule | Given | When | Then |
| --- | --- | --- | --- |
| Task-0 copy | Loader at `$0800` | Copy `$1000–$37FF` → `$B800–$DFFF` | Memory at `$B800–$D870` contains DOS/65 code |
| Task-1 copy | Loader at `$0800` | Copy `$4000–$5FFF` → task-1 `$C000–$DFFF` via task switch | Task-1 physical `$10000–$11FFF` holds driver code |
| Far call | Far-call stub at `$FFF0` invoked | Switches to task 1; calls `$C000` dispatcher | Returns to task-0 caller; A register preserved |
| SIM cold init | DOS/65 cold boot | Device init loop runs | No abort; all absent devices return without fault |
| DOS prompt | Cold boot completes | Serial console output observed | `DOS/65` banner and `A>` prompt visible |
| Prompt responsiveness | `A>` visible | `A:` typed and sent | Echo received; no hang or fault |

### M5 — DOS/65 disk I/O

| Rule | Given | When | Then |
| --- | --- | --- | --- |
| DIR listing | DOS prompt active | `DIR A:` command issued | CP/M directory entries from disk image listed |
| Program load | Valid `.COM` file on disk | File loaded and executed | Program runs without hang |
| Write–read roundtrip | File written to disk | Same file subsequently read | Content identical to written data |
| Drive E failure | Drive E accessed | SIM read/write for drive E | Failure returned; no crash or hang |
| Bad-sector prompt | XT-IDE stub returns bad-sector | SIM read failure | `BAD SECTOR` prompt; Return → continue; other key → warm boot |
| Drive B isolation | Write to drive B | B-range LBAs accessed | A-range LBAs unchanged; data not corrupted |

### M6 — RTC and configuration hardening

| Rule | Given | When | Then |
| --- | --- | --- | --- |
| RTC read | RTC policy configured | DOS time/date command issued | Plausible date matching configured policy returned |
| RTC write sequence | Control sequence written | `$02`/`$00`/`$00`/`$01`/`$05`/`$04` written to RTC control | No fault; clock advances afterward |
| Absent CH375 | CH375 not present | `C:` access attempted | Failure returned; DOS prompt returns; no crash |
| Keyboard self-test | Multi-I/O present | `$AA` command written | `$55` response accepted; init does not hang |
| Open-bus read | `$EFA0` read | Supermon memory examine | Returns configured open-bus byte |
| Config file boot | Non-default config file supplied | Emulator started with custom K1 bank, open-bus `$EA`, host-clock RTC | All settings applied; emulator starts without error |

## Acceptance Criteria

### REQ-M1 — Core CPU, flat address space, ACIA serial output

1. Reset vector fetch from `$FFFC–$FFFD` returns `$00 $F0` (Base bank).
2. BIOS startup SEI/CLD/LDX/TXS sequence completes without CPU fault.
3. SERIALINIT writes `$00`→`$EF85`, `$0B`→`$EF86`, `$1E`→`$EF87` in order; ACIA does not crash.
4. Startup banner characters appear on emulator serial output (UART TDRE polling exits normally).
5. BRK at end of Base reset path reaches Supermon; serial prompt `>` is received.
6. `G F000` in Supermon re-runs the reset path and re-emits the banner without hanging.

### REQ-M2 — MMU: 64 tasks, BIOS INITPAGES, task switching

1. INITPAGES completes: reading `$EFE4` after BIOS init returns `$00` (task 0) with bit 7 set.
2. Task-0 map round-trip: write 16 known bytes to `$EFD0–$EFDF` for task 0; read back identical bytes.
3. Task-1 logical `$C000–$DFFF` addresses physical `$10000–$11FFF` (not task-0's `$0C000–$0DFFF`).
4. Identity task-0 map: CPU `$1234` reaches physical `$01234`; `$E900` reaches I/O overlay; `$F000` reaches ROM.
5. After INITPAGES, banner still prints (MMU enabled, task 0 active, all ROM/UART addresses preserved).
6. Task mask: Supermon memory examine of `$EFE4` after writing `$FF` to `$EFE0` shows `$3F` in bits 5:0.

### REQ-M3 — XT-IDE emulation and VIDEO ROM 60-sector boot

1. VIDEO ROM reset vector fetch returns `$00 $F0` correctly.
2. Probe writes to `$E300–$E330` do not crash the emulator or corrupt disk state.
3. SET FEATURES `$EF` succeeds: BSY clears; DRQ and ERR absent.
4. All 60 sector reads succeed: no serial error-path output.
5. Serial banner appears (VIDEO bank also initializes UART).
6. Execution reaches `$0800`: debugger or logging confirms PC=`$0800` after the final sector copy.

### REQ-M4 — DOS/65 cold boot to prompt

1. Loader task-0 copy: memory at `$B800–$D870` contains DOS/65 code after `JMP $0800` executes.
2. Loader task-1 copy: task-1 physical `$10000–$11FFF` contains banked driver code.
3. Far call `$FFF0` → dispatcher `$C000` in task 1 → returns to task-0 caller with A preserved.
4. SIM cold init loop completes: no device-init failure halts execution.
5. DOS/65 cold-boot banner and prompt appear on emulator serial console (`DOS/65` and `A>`).
6. Typing `A:` at the prompt echoes and does not hang or fault.

### REQ-M5 — DOS/65 disk read/write and directory listing

1. `DIR A:` on the DOS prompt lists directory entries from the CP/M filesystem on the XT-IDE disk image.
2. A valid `.COM` program loads and runs without hanging.
3. A write then read-back of a small file produces identical content.
4. Accessing drive E returns failure; no crash or hang.
5. Injecting a bad-sector response causes the `BAD SECTOR` prompt; pressing Return continues; any other key warm-boots to `A>`.
6. Drive B accesses LBA range `$4100–$81FF`; a write to B does not corrupt A's data.

### REQ-M6 — RTC, optional peripherals, and configuration hardening

1. DOS/65 RTC read call: the time/date command returns a plausible date matching the configured RTC policy.
2. RTC write sequence: firmware `$02`/`$00`/`$00`/`$01`/`$05`/`$04` control sequence executes without fault; clock advances afterward.
3. Absent CH375: DOS `C:` access returns failure; no crash or hang; DOS prompt returns.
4. Keyboard self-test: injecting `$55` response to `$AA` command passes controller init; Multi-I/O init does not hang. Verified at the register level: emulator writes `$AA` to `KBD_CMD` (`$E3FF`) and reads `$55` back from `KBD_DAT` (`$E3FE`), matching real firmware's port usage (`bios_multi.asm:173-176,292,361`; see `specifications/multio-card-investigation.md` mc-zrr fix). Verified via full firmware execution (mc-hpg): `$E3FF` now models bit1 (busy, forced clear) and bit0 (data-pending, tracks the queued self-test response), so real firmware's `KBD_PUTCMD`/`KBD_GETDATA` polling loops (`bios_multi.asm:269-363`) resolve on genuine handshake state instead of the raw open-bus byte. Real DOS/65 boot against `rom.hex`/`disk.img` (`emulator/tests/m4_dos_boot_gate.rs::m4_real_boot_far_call_and_sim_init`) now prints `KBD: INITIALIZED.` instead of `KBD: VT82C42 WRITE TIMEOUT.`; the test asserts the transcript never contains `VT82C42`.
5. Open-bus reads from `$EFA0–$EFCF` return the configured value (verified via Supermon memory examine).
6. Emulator starts correctly from a config file specifying non-default K1 bank, open-bus `$EA`, and host-clock RTC.

## Coverage Matrix

| ID | Status |
| --- | --- |
| REQ-M1 | covered |
| REQ-M2 | covered |
| REQ-M3 | covered |
| REQ-M4 | covered |
| REQ-M5 | covered |
| REQ-M6 | covered |

## Out Of Scope

- Video card (memory-mapped display, pages `$F8–$F9`): present in the spec but not required for DOS/65 serial operation; a separate milestone may be added after M6.
- Cycle-accurate CPU timing: baud-rate emulation and startup delay loops do not need wall-clock accuracy.
- Floppy drive: the DOS/65 build treats it as a no-op; no implementation needed.
- ATX power control and physical reset-switch emulation.
- W65C51N transmit-ready silicon errata: deferred until hardware measurement confirms which UART part is installed.
- CH375 vs CH376 command-set differences beyond the absent-device safe path.
- ESP network functionality: only the absent-device safe path is required.
- Multi-I/O beyond keyboard self-test (`$AA`/`$55`): mouse port, IRQ routing, and remaining status bits are out of scope.

## Open Questions

These unknowns are taken from `system-reference.md §9` (priority P0 and P1). Each must be resolved as a named emulator configuration knob rather than a hard-coded default.

**OQ-R0.1: CPU part (NMOS 6502 vs 65C02 variant).** Affects undocumented opcodes, decimal-mode behavior, reset cycle count, and interrupt timing. Minimum for M1: standard documented 6502 opcodes; CPU subtype must be a configuration option.

**OQ-R0.2: CPU oscillator frequency.** All timing-sensitive behavior (UART baud clock, RTC timebase, startup delay, XT-IDE wait-state adequacy) is unknown. Emulator does not need wall-clock accuracy but must expose a configurable clock frequency for future use.

**OQ-R0.3: MMU power-on map contents.** Map SRAM is volatile; contents before BIOS writes tasks 0/1 are indeterminate. Emulator must never promise specific pre-INITPAGES values; indeterminate behavior may be simulated with a random or configurable fill.

**OQ-R0.4: Physical default K1 ROM bank.** The emulator cannot infer which bank (Base vs VIDEO) was shipped or jumpered. A user-selectable configuration option is required; no hardcoded default.

**OQ-R0.5: Open-bus read value for unmapped addresses.** Reads to `$EFA0–$EFCF`, `$EFF0–$EFFF`, unassigned MMU offsets, and absent physical pages return unknown data. A named, configurable open-bus policy is required; `$00` is not an acceptable implicit default.

**OQ-R0.6: Shadow-address strap setting (P1 `SHADOW ADDR`).** The installed jumper position is unrecorded. Default must match the firmware-compatible low-page assignment; the strap position must be configurable.

**OQ-R0.7: ROM/I/O overlay precedence over arbitrary MMU mappings.** Only the identity-map case (task-0 logical pages `$E` and `$F` map to physical I/O and ROM) is confirmed safe. Whether the board forces I/O/ROM decode regardless of MMU mapping at those physical pages is unknown. Implement identity-map behavior only; make overlay-always policy a configuration option pending hardware validation.

**OQ-R1.1: ACIA variant and reference clock.** Baud-rate timing, W65C51N transmit-ready silicon errata, and IRQ output wiring are unconfirmed. Emulator must model the confirmed polling contract; expose ACIA variant as a configurable option.

**OQ-R1.4: Modem-input defaults (CTS, DCD, DSR).** CTS is believed to be forced high by a board jumper; a wrong default can deadlock UART transmit. CTS default must be configurable; default to CTS-asserted (high) for firmware compatibility.
