---
schema: gc.build.plan.v1
workflow:
  id: pe-w1b
  formula: build-from-requirements
methodology:
  pack: gascity
  name: build-from-requirements
producer:
  formula: build-from-plan-base
  stage: plan
  attempt: 1
status: approved
trace:
  upstream:
    - path: plans/pc6502-emulator-milestones/requirements.md
      hash: "sha256:4787796dc14e291da689992b7ce9c9defd2fc54b86d18e85761fbbfbb9d4e729"
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

# Implementation Plan: PC6502 Emulator — Six-Milestone Build

## Summary

Build a headless, command-line software emulator for the PC6502 single-board
computer (6502-family SBC) in six milestones. Each milestone is gated by
observable serial output produced by the actual `rom.hex` and DOS/65 disk
images — not by unit tests alone. The emulator must run on Linux without
access to physical hardware.

The six milestones add capability incrementally:

| Milestone | Gate observable |
|---|---|
| M1 | Supermon `>` prompt on serial after reset |
| M2 | INITPAGES completes; MMU map round-trip passes; banner still prints |
| M3 | 60 XT-IDE sectors transfer; PC = `$0800` confirmed |
| M4 | `DOS/65` banner and `A>` prompt on serial |
| M5 | `DIR A:` lists CP/M directory entries |
| M6 | RTC call returns plausible date; emulator starts from config file |

All hardware unknowns (P0 and P1) are exposed as named configuration knobs.
No hardcoded defaults are permitted for any unknowns listed in OQ-R0.1 through
OQ-R1.4.

---

## Current System

No emulator source exists. The repository contains:

- `PC6502_firmware_source/` — 6502 BIOS assembly and linker artifacts including
  `rom.hex` (8 KiB Intel HEX, two 4 KiB banks)
- `DOS65_OS/` — DOS/65 OS sources and pre-built disk images; some C utility
  sources confirm the developer ecosystem uses C/6502 assembly
- `specifications/` — five authoritative investigation documents synthesized
  from firmware evidence (no schematic or logic-analyzer data available)
- `plans/pc6502-emulator-milestones/requirements.md` — approved requirements
  with six milestone REQ IDs and full acceptance-criteria tables

No physical PC6502 board is accessible. Emulator correctness must be validated
by running the actual ROM and disk images and observing the specified serial
output at each milestone gate.

---

## Proposed Implementation

### Technology stack

**Language:** Rust (stable toolchain, Cargo workspace).

Rationale: Rust is the appropriate choice for a cycle-by-cycle hardware
emulator. Its ownership model prevents the class of aliasing and
use-after-free bugs common in C emulators; its zero-cost abstractions keep
the emulation loop efficient; and its trait system maps cleanly onto the bus
peripheral interface. No other dependency with emulator source exists in the
repository, so there is no incumbent language to match.

**Output format:** stdout for UART TX; stdin or a named pipe for UART RX
injection; no GUI or video output required through M6.

**Configuration:** TOML config file. Every P0/P1 unknown is a named key with
a documented range of valid values and a *required* comment explaining what
hardware evidence would resolve it. No key may silently default to an
unrecorded value.

### Repository layout

```
emulator/                      ← new Cargo workspace root
  Cargo.toml
  Cargo.lock
  config/
    default.toml               ← required base config; documents all knobs
  src/
    main.rs                    ← CLI: parse args, load config, load ROM, run
    config.rs                  ← typed config struct; all P0/P1 knobs
    emulator.rs                ← Machine struct: owns CPU, Bus, peripherals
    cpu/
      mod.rs                   ← Cpu struct, step(), interrupt dispatch
      opcodes.rs               ← instruction decode and execute table
      flags.rs                 ← flag register helpers
    bus.rs                     ← AddressBus: decode, MMU dispatch, open-bus
    mmu.rs                     ← Mmu: 64-task map store, edit window, registers
    acia.rs                    ← Acia: 6551 TX/RX model
    xt_ide.rs                  ← XtIde: ATA registers, sector buffer, disk image
    disk.rs                    ← DiskImage: load raw image, sector read/write
    rtc.rs                     ← Rtc: 72421/72423 nibble model, three policies
    rom.rs                     ← load_hex(): parse Intel HEX, split banks
    peripherals.rs             ← absent-device stubs (CH375, ESP, Multi-I/O)
  tests/
    m1_serial_gate.rs          ← integration: run until Supermon prompt
    m2_mmu_gate.rs
    m3_xt_ide_gate.rs
    m4_dos_boot_gate.rs
    m5_disk_io_gate.rs
    m6_rtc_config_gate.rs
```

The `emulator/` directory is placed at the repository root alongside
`specifications/` and `plans/`.

### Module responsibilities and acceptance linkage

#### config.rs — P0/P1 configuration knobs

Each knob is a named field in `Config`. No field may have an implicit default
that hides a hardware unknown. All fields must be present in `default.toml`
with a comment citing the open question ID.

| Config key | OQ ref | Valid values | Notes |
|---|---|---|---|
| `cpu_subtype` | OQ-R0.1 | `"nmos"` \| `"65c02"` | Affects decimal-mode and undocumented opcodes |
| `cpu_hz` | OQ-R0.2 | integer (Hz) | Wall-clock accuracy not required; future use |
| `mmu_power_on_fill` | OQ-R0.3 | `"random"` \| `"zero"` \| 0x00–0xFF | Contents before INITPAGES |
| `rom_bank` | OQ-R0.4 | `"base"` \| `"video"` | Selects first or second 4 KiB from rom.hex |
| `open_bus` | OQ-R0.5 | 0x00–0xFF | Returned for all unmapped reads |
| `shadow_addr` | OQ-R0.6 | `"low"` \| `"high"` | Shadow-address strap position |
| `io_rom_always` | OQ-R0.7 | `true` \| `false` | Whether I/O/ROM decode overrides arbitrary MMU mappings |
| `acia_cts_default` | OQ-R1.4 | `true` \| `false` | CTS-force-high jumper state |
| `rtc_policy` | TS-6 | `"host"` \| `"epoch"` \| `"fixed"` | Clock source for RTC reads |
| `rtc_epoch` | TS-6 | ISO-8601 datetime string | Used only when `rtc_policy = "fixed"` |

#### cpu/mod.rs — 6502 CPU core (M1 gate)

Implements the `Cpu` struct with:

- `step(&mut self, bus: &mut Bus) -> u8` — execute one instruction, return
  cycle count
- All documented 6502 opcodes with correct flag semantics, stack pointer
  behavior, and address-mode wrapping
- Interrupt dispatch: RESET, NMI, IRQ, BRK — correct vector fetch addresses
  (`$FFFA–$FFFF`)
- NMOS vs 65C02 subtype branching where behavior differs (decimal mode flag
  clearing on RESET in 65C02; ROR and undocumented opcodes for NMOS)
- No cycle-accurate timing beyond correct cycle counts returned by `step()`

Acceptance links: BR-1 (reset vector decode), TS-1, REQ-M1 items 1–6.

#### bus.rs — address bus and memory decode (M1 gate)

Implements the `Bus` struct with:

- Flat 64 KiB logical decode in MMU-disabled mode
- I/O overlay priority: `$E000–$EFFF` routes to the I/O register block before
  any RAM backing
- ROM write protection: writes to `$F000–$FFFF` are silently discarded
- Configurable `open_bus` byte returned for all unmapped reads (`$EFA0–$EFCF`,
  `$EFF0–$EFFF`, unassigned MMU offsets, physical holes)
- Peripheral dispatch table keyed on decoded I/O address ranges
- `read(addr: u16) -> u8` and `write(addr: u16, val: u8)` as the CPU
  interface; MMU translation inserted between logical and physical when enabled

Acceptance links: BR-6 (open-bus), BR-8 (ROM write protect), BR-9 (physical
holes), TS-2, TS-8.

#### mmu.rs — 64-task MMU (M2 gate)

Implements the `Mmu` struct with:

- 1024-byte map store (64 tasks × 16 entries of 1 byte each)
- `translate(logical_page: u8, task: u8) -> u8` — physical page lookup
- Edit window at `$EFD0–$EFDF`: reads and writes for the selected setup task
- Control registers at `$EFE0–$EFE7`:
  - `$EFE0`: write active task (lower 6 bits)
  - `$EFE1`: write setup task for edit window
  - `$EFE2`: write MMU enable (`0` = disabled, `1` = enabled)
  - `$EFE4`: read active task (bits 5:0) and enable status (bit 7)
  - `$EFE6`: read "hit ISA TC bit" (stub; semantics unknown — return 0)
  - `$EFE7`: read current I/O page (lower 4 bits)
- Power-on map contents driven by `mmu_power_on_fill` config
- Task-0 alias compatibility option (see hardware-spec §5 warning)
- Task mask: writing `$FF` to `$EFE0` sets active task to `$3F`

Acceptance links: BR-3 (task mask), BR-4 (enable), TS-3, REQ-M2 items 1–6.

#### acia.rs — 6551-compatible UART (M1 gate)

Implements the `Acia` struct with:

- Four registers at `$EF84–$EF87`:
  - `$EF84`: data register (TX write; RX read)
  - `$EF85`: status register (bit 4 = TDRE always 1; bit 3 = RDRF set on
    injected byte); write `$00` to perform programmed reset
  - `$EF86`: command register (read/write; controls parity, echo, IRQ)
  - `$EF87`: control register (read/write; baud-rate divisor, word length)
- TX: every write to `$EF84` emits the byte on stdout (or configured TX sink)
- RX: RDRF set when an externally injected byte is available; cleared on read
- CTS default driven by `acia_cts_default` config knob
- No IRQ generation required through M6 (polling path only)

Acceptance links: BR-2 (init sequence), TS-4, REQ-M1 items 3–4.

#### xt_ide.rs + disk.rs — XT-IDE controller and disk image (M3 gate)

`DiskImage` loads a raw sector image and exposes:
- `read_sector(lba: u32) -> [u8; 512]`
- `write_sector(lba: u32, data: &[u8; 512])`

`XtIde` implements ATA registers at `$E300–$E30E` (even-spaced):
- BSY, DRQ, ERR status bits
- Sector read command `$20`: BSY set, DRQ set when buffer ready, 512-byte
  transfer via data register reads
- SET FEATURES `$EF`: BSY clears; DRQ and ERR absent
- IDENTIFY `$EC`: returns a minimal 512-byte IDENTIFY block
- Probe write tolerance: writes of `$FF` and `$00` to `$E300–$E330` must not
  crash or corrupt disk state
- LBA drive-B isolation: B-range LBAs (`$4100–$81FF`) are disjoint from
  A-range; a write to B does not corrupt A

Acceptance links: BR-5 (probe tolerance), TS-5, REQ-M3 items 2–4, REQ-M5
items 1–6.

#### rtc.rs — RTC model (M6 gate)

Implements the `Rtc` struct with:
- 16 nibble-only registers at `$EF90–$EF9F` (only low nibble of each byte is
  meaningful)
- Control registers at offsets `$0D–$0F`
- Three clock sources selected by `rtc_policy`:
  - `host`: reads from the Linux wall clock
  - `fixed`: reads a constant value from `rtc_epoch` config
  - `epoch`: reads from a persistent image file (updated on writes)
- STOP/RESET bit behavior: writing the documented control sequence freezes then
  restarts the counter without fault

Acceptance links: TS-6, REQ-M6 items 1–2.

#### peripherals.rs — absent-device stubs (M4 gate)

Safe no-op stubs for devices that must not crash or hang:
- CH375 at `$E260–$E261`: reads return `open_bus`; writes are silently
  discarded
- Dual ESP at `$E100–$E102`: same
- Multi-I/O at `$E3FE–$E3FF`: keyboard self-test returns `$55` in response to
  `$AA`; all other reads return `open_bus`
- Unmapped expansion `$E000–$E2FF` and `$E303–$E32F` and `$E330–$EF7F`
  (excluding documented ranges): reads return `open_bus`; writes are silently
  discarded

Acceptance links: BR-7 (absent-device safety), TS-8, REQ-M4 item 4,
REQ-M6 items 3–4.

### Milestone build order

Each milestone adds modules or capability on top of the previous and must
pass its gate observable before the next begins.

**M1 — CPU + ACIA, flat 64 KiB, Supermon prompt**

Modules: `cpu/`, `bus.rs` (MMU disabled), `acia.rs`, `rom.rs`, `config.rs`,
`main.rs`

Implementation steps:
1. Parse `rom.hex` into two 4 KiB banks; select bank via `rom_bank` config
2. Implement `Cpu::step()` for all documented 6502 opcodes; RESET vector fetch
   returns `$F000` from the configured bank
3. Implement flat 64 KiB bus: RAM `$0000–$DFFF`, I/O overlay `$E000–$EFFF`,
   ROM `$F000–$FFFF`
4. Implement ACIA at `$EF84–$EF87`: TX to stdout, TDRE always set, programmed
   reset no-op
5. Wire RESET: fetch vector at `$FFFC–$FFFD`, set PC, clear interrupt-disable
6. Run until Supermon `>` appears on stdout; confirm `G F000` re-runs banner

Gate observable: `>` on serial console after reset.

**M2 — MMU: INITPAGES, task switching, address translation**

Modules: `mmu.rs`; extend `bus.rs` with MMU translation path

Implementation steps:
1. Add `Mmu` struct with 1024-byte map store and edit-window at `$EFD0–$EFDF`
2. Wire MMU enable/disable via `$EFE2`; disabled = identity map
3. Translate CPU logical address through active-task map when enabled
4. Implement active/setup task registers at `$EFE0–$EFE1`
5. Implement readback at `$EFE4`
6. Initialize map store with `mmu_power_on_fill` value
7. Run BIOS INITPAGES; confirm `$EFE4` readback and banner still prints

Gate observable: INITPAGES completes; `$EFE4` shows task 0, enable bit set;
serial banner still printed.

**M3 — XT-IDE emulation and VIDEO ROM 60-sector boot**

Modules: `xt_ide.rs`, `disk.rs`; extend `bus.rs` for XT-IDE decode

Implementation steps:
1. Load raw disk image file named by config key `disk_image`
2. Implement XT-IDE ATA register block at `$E300–$E30E`
3. Implement sector-read state machine: write LBA, issue READ SECTORS `$20`,
   transfer 512 bytes via data register
4. Handle probe writes to `$E300–$E330` without fault
5. Switch `rom_bank` to `video` in config; confirm reset vector still returns
   `$F000` (both banks share same reset entry)
6. Run VIDEO ROM boot path; count 60 sector transfers; confirm PC = `$0800`
   via trace log

Gate observable: 60 sector reads succeed without error; PC = `$0800` logged.

**M4 — DOS/65 cold boot to `A>` prompt**

Modules: extend `bus.rs` for task-switching during loader; `peripherals.rs`
stubs

Implementation steps:
1. Ensure loader task-0 copy from `$1000–$37FF` to `$B800–$DFFF` succeeds
2. Ensure loader task-1 copy to physical `$10000–$11FFF` uses task-1 map
3. Wire far-call stub at `$FFF0`: switches to task 1, calls `$C000`, returns
   to task-0 caller with A preserved
4. Stub all absent devices (CH375, ESP, Multi-I/O) as safe no-ops
5. Run cold boot; observe `DOS/65` banner and `A>` on serial

Gate observable: `DOS/65` and `A>` on serial console.

**M5 — DOS/65 disk read/write and directory listing**

Modules: extend `xt_ide.rs` for WRITE SECTORS; `disk.rs` write path

Implementation steps:
1. Implement WRITE SECTORS command: accept 512-byte transfer, write to disk
   image
2. Implement IDENTIFY response (minimal 512-byte block; model string can be a
   fixed ASCII string)
3. Wire bad-sector injection path: configurable per-LBA error return
4. Drive B LBA isolation: verify B-range writes do not touch A-range sectors
5. From `A>`, issue `DIR A:`; confirm directory entries listed
6. Load and run a `.COM` file; confirm program runs without hang
7. Write then read back a small file; confirm content identity

Gate observable: `DIR A:` lists CP/M directory entries.

**M6 — RTC and configuration hardening**

Modules: `rtc.rs`; extend `config.rs` for all remaining knobs and config-file
loading

Implementation steps:
1. Implement RTC nibble model at `$EF90–$EF9F` with three clock policies
2. Implement RTC control sequence without fault
3. Confirm Multi-I/O keyboard self-test: `$AA` → `$55`
4. Confirm open-bus reads from `$EFA0–$EFCF` return configured value
5. Implement config-file boot: load TOML from `--config` CLI arg; apply all
   non-default settings; confirm emulator starts without error
6. Integration test: start with `rom_bank = "video"`, `open_bus = 0xEA`,
   `rtc_policy = "host"`; confirm all settings applied and boot succeeds

Gate observable: RTC read returns plausible date; emulator starts from
config file with non-default settings.

---

## Assumptions

**A-1:** The `rom.hex` file at the repository root is the authoritative 8 KiB
ROM image. The Base bank occupies the first 4 KiB of the HEX data region and
the VIDEO bank the second 4 KiB, consistent with the board's K1 jumper
description. If the ordering differs, the `rom_bank` config knob will be
the mechanism to test both.

**A-2:** All documented 6502 opcodes use NMOS addressing and flag behavior as
the default. The `cpu_subtype = "65c02"` path activates 65C02-specific
differences only for opcodes where the difference is documented and confirmed
to matter for the boot path (decimal-mode flag clear on RESET; `ROR`
correction). Undocumented NMOS opcodes are not required for any milestone gate.

**A-3:** The 512-byte ATA sector is the correct transfer unit for XT-IDE.
`disk_image` is a raw flat binary with LBA 0 at byte offset 0.

**A-4:** The DOS/65 disk image uses CP/M-compatible directory structure at
the beginning of the A-drive partition (LBA `$0000–$40FF`). B-drive occupies
`$4100–$81FF` as stated in the requirements.

**A-5:** The BIOS UART TX polling loop exits when TDRE (bit 4 of `$EF85`) is
set. The emulator holds TDRE permanently set (always ready) since no baud
clock is modeled. This is sufficient for all milestone gates.

**A-6:** The far-call stub at `$FFF0` is implemented in the ROM. The emulator
does not need to synthesize this stub; it executes whatever the ROM bank
contains at that address.

**A-7:** No IRQ or NMI is required for any milestone gate through M6. The
`SEI` in the BIOS startup sequence prevents interrupts; all I/O is polled.

---

## Risks

**R-1 (HIGH): ROM bank layout unknown at byte level.** The K1 bank selection
mechanism is documented (jumper selects one of two 4 KiB regions), but the
exact byte offsets within `rom.hex` for Base vs VIDEO bank are not confirmed.
If the HEX file does not split cleanly at 4 KiB from the data region start,
the `rom_bank` config knob will allow testing both orderings. Mitigation:
parse HEX data records, log the detected address ranges, and let config select
which 4 KiB window serves as the CPU ROM.

**R-2 (HIGH): MMU power-on map indeterminate.** BIOS INITPAGES assumes a
known starting state only in the tasks it explicitly writes. If other tasks
have stale values from a warm reboot, task-switching code might fault. The
`mmu_power_on_fill` knob and deterministic ordering in INITPAGES mitigate this
for the cold-boot case.

**R-3 (MEDIUM): XT-IDE register spacing.** The spec says `$E300–$E30E` with
"even-spaced ATA registers plus separate low/high data bytes." The exact
decode layout for the low/high data split is not confirmed beyond the
firmware's access pattern. Misdecoding stalls the 60-sector read without
a visible error. Mitigation: instrument `xt_ide.rs` with trace logging of
every register access and compare against the 60-sector read sequence.

**R-4 (MEDIUM): Far-call stub encoding in ROM.** The far-call stub at `$FFF0`
in the ROM enables task switching. If the ROM contains a different encoding
than the loader expects, the call returns to the wrong task or address.
Mitigation: disassemble the ROM around `$FFF0` before M4 implementation and
confirm the stub sequence matches the BIOS source.

**R-5 (LOW): CTS default deadlock.** If `acia_cts_default = false` and the
firmware's TX poll checks CTS before TDRE, transmit stalls silently. The
default in `default.toml` must be `acia_cts_default = true`. This is a
configuration error, not a code defect.

**R-6 (LOW): Open-bus value matters at startup.** If the BIOS reads an open-bus
address and branches on the result (e.g., as a presence check), the configured
open-bus value may change the execution path. Expose `open_bus` prominently in
`default.toml` and document that `$EA` (NOP opcode) is a reasonable first
choice, but the value is hardware-unknown.

---

## Non-Goals

The following are explicitly out of scope for this plan, matching the
requirements out-of-scope list:

- **Video card emulation** — physical pages `$F8–$F9`, mode registers, color
  and text memory. A stub that returns `open_bus` for those physical pages
  is sufficient.
- **Cycle-accurate CPU timing** — baud-rate clock and startup delay loops do
  not need wall-clock accuracy. `cpu_hz` is stored but not enforced.
- **Floppy drive** — DOS/65 treats floppy as a no-op; no implementation needed.
- **ATX power control and reset-switch emulation.**
- **W65C51N transmit-ready silicon errata** — deferred until hardware
  measurement confirms the UART part.
- **CH375 vs CH376 command-set differences** — only the absent-device safe
  path is required.
- **ESP network functionality** — only absent-device safe path required.
- **Multi-I/O beyond keyboard self-test** — mouse port, IRQ routing, remaining
  status bits are out of scope.
- **IRQ and NMI interrupt delivery** — the SEI-guarded polling model covers
  all six milestone gates without interrupt support.

---

## Verification

Each milestone's verification strategy relies on running the actual ROM and
disk images. No mock substitutes for ROM content or disk image data are
permitted as the sole gate.

### M1 verification

1. Load `rom.hex` with `rom_bank = "base"`, `cpu_subtype = "nmos"`,
   `acia_cts_default = true`, `open_bus = 0xEA`.
2. Assert that the first bus read at `$FFFC–$FFFD` returns `$00 $F0`.
3. Run the emulator; capture stdout.
4. Assert that the Supermon prompt `>` appears in captured output within a
   bounded number of CPU cycles (10M cycles max).
5. Inject `G F000\r`; assert that the banner re-appears without a second `>`
   timeout.

### M2 verification

1. Continue from M1 config; enable MMU (firmware INITPAGES runs automatically).
2. After INITPAGES, assert `read($EFE4) & 0xBF == 0x80` (enable bit set, task
   = `$00`).
3. Write 16 known bytes to `$EFD0–$EFDF` for task 0; read back; assert
   identical.
4. Switch to task 1; assert that a read at logical `$C000` translates to
   physical `$10000`.
5. Assert banner still printed on stdout (no hang after MMU enable).
6. Write `$FF` to `$EFE0`; assert `read($EFE4) & 0x3F == 0x3F`.

### M3 verification

1. Switch to `rom_bank = "video"`; assert reset vector still returns `$F000`.
2. Issue probe writes `$FF`/`$00` to `$E300–$E330`; assert emulator does not
   crash and disk image bytes at LBA 0 are unchanged.
3. Run VIDEO ROM boot path; assert no error-path output on stdout.
4. Instrument `xt_ide.rs` to count completed sector transfers; assert count
   reaches 60.
5. Assert CPU PC equals `$0800` after the 60th sector transfer (via trace log
   or debug hook).

### M4 verification

1. Run VIDEO ROM cold-boot path through DOS/65 loader.
2. Assert memory at physical `$B800–$D870` (task-0 map) is non-zero after
   loader task-0 copy.
3. Assert physical `$10000–$11FFF` is non-zero after loader task-1 copy.
4. Assert stdout contains the substring `DOS/65` followed by `A>`.
5. Inject `A:\r`; assert echo received; assert no timeout or fault.

### M5 verification

1. From `A>` state, inject `DIR A:\r`; capture stdout.
2. Assert at least one CP/M directory entry pattern appears in output.
3. Load a known `.COM` file from disk; assert program prompt or output
   appears and emulator does not hang.
4. Write a small test file; read it back; assert byte-for-byte identity.
5. Access drive E; assert failure response; assert `A>` returns.
6. Inject a bad-sector error for a known LBA; assert `BAD SECTOR` in stdout;
   inject Return; assert `A>` returns.
7. Write to a B-drive LBA; read A-drive LBA 0; assert A-drive content
   unchanged.

### M6 verification

1. Configure `rtc_policy = "host"`; run DOS/65 time/date command; assert
   output contains a plausible year (2020–2040).
2. Execute RTC write sequence `$02/$00/$00/$01/$05/$04`; assert no fault.
3. Configure `rtc_policy = "fixed"` with a known epoch; assert returned
   date matches.
4. Access drive C (CH375 absent); assert failure, no crash, `A>` returns.
5. Inject `$AA` to Multi-I/O keyboard; assert `$55` response; assert init
   does not hang.
6. Read `$EFA0`; assert returned value equals `open_bus` config value.
7. Start emulator with `--config custom.toml` containing
   `rom_bank = "video"`, `open_bus = 0xEA`, `rtc_policy = "host"`; assert
   boot succeeds with no errors.

### Regression policy

After each milestone gate passes, the corresponding integration test in
`tests/mN_*_gate.rs` must be committed and must remain green throughout all
subsequent milestones. A failing M1 gate during M3 work is a blocker.

### Unresolved ambiguities recorded here (no open questions deferred)

OQ-R0.1 through OQ-R1.4 are all resolved as runtime configuration knobs
rather than implementation-time decisions. No hardware evidence is needed to
begin implementation; each knob will be validated against physical hardware
when the board becomes accessible.
