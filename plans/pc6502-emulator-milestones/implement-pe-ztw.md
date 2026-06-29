---
schema: gc.build.implementation-summary.v1
workflow:
  id: pe-d57
  formula: do-work
methodology:
  pack: gascity
  name: build-basic
producer:
  formula: do-work
  stage: implement
  attempt: 1
status: approved
trace:
  upstream:
    - path: beads/pe-ztw
      hash: bead:pe-ztw
    - path: emulator/Cargo.toml
      hash: sha256:8e9a1cf858a891d734cb50834d41ec9f74eb047444b48236982e50695a875c3e
    - path: emulator/config/default.toml
      hash: sha256:c557acdfe4d2b4e4a795bd1c68cfce5d05135c70b94c71ec52d07a252a5f39d8
    - path: emulator/src/main.rs
      hash: sha256:9426bfada2717757259c77a871d045219972fa127b939598c8b0475edea05367
  coverage:
    - id: REQ-WI-SETUP-01
      status: covered
    - id: REQ-WI-SETUP-02
      status: covered
    - id: REQ-WI-SETUP-03
      status: covered
    - id: REQ-WI-SETUP-04
      status: covered
    - id: REQ-WI-SETUP-05
      status: covered
---

## Summary

Initialized the `emulator/` Cargo workspace at the repository root for the PC6502 emulator project. Created all required source skeleton files, configuration, gate test placeholders, and `Cargo.lock` (committed for reproducible builds). `cargo check` passes with zero errors.

## Intended Behavior

The workspace provides the skeleton on which all subsequent milestone work (WI-M1 through WI-M6) will be built:

- `emulator/Cargo.toml` — workspace root with `[workspace]` and `[package]` for the `emulator` binary; dependencies: `serde` and `toml`.
- `emulator/config/default.toml` — runtime-settable knobs for all P0/P1 open questions: `cpu_subtype`, `cpu_hz`, `mmu_power_on_fill`, `rom_bank`, `open_bus`, `shadow_addr_low`, `io_rom_always`, `acia_variant`, `acia_cts_default`, `disk_image`, `rom_hex`, `rtc_policy`, `rtc_epoch`.
- `emulator/src/` — stub module tree: `main.rs`, `config.rs`, `emulator.rs`, `bus.rs`, `acia.rs`, `mmu.rs`, `xt_ide.rs`, `disk.rs`, `rtc.rs`, `rom.rs`, `peripherals.rs`, `cpu/mod.rs`, `cpu/opcodes.rs`, `cpu/flags.rs`.
- `emulator/tests/` — six `#[ignore]`-gated test files, one per milestone gate (M1–M6).
- `emulator/.gitignore` — excludes `target/`.

## Changed Files

| File | Change |
| --- | --- |
| `emulator/.gitignore` | new — excludes build artifacts |
| `emulator/Cargo.toml` | new — workspace + package manifest |
| `emulator/Cargo.lock` | new — pinned dependency lockfile |
| `emulator/config/default.toml` | new — all P0/P1 config knobs with explanatory comments |
| `emulator/src/main.rs` | new — binary entry point, loads config and calls `Machine::run()` |
| `emulator/src/config.rs` | new — `Config` struct with all knobs, TOML-deserialization via serde |
| `emulator/src/emulator.rs` | new — `Machine` struct: owns CPU + bus, drives reset and run stub |
| `emulator/src/bus.rs` | new — `Bus` struct: RAM, ROM, ACIA, MMU, XT-IDE, RTC, Peripherals |
| `emulator/src/acia.rs` | new — 6551-compatible ACIA: TX→stdout, RX inject, status bits |
| `emulator/src/mmu.rs` | new — 64-task MMU with 1 KiB map store, edit window, control regs |
| `emulator/src/xt_ide.rs` | new — XT-IDE controller stub (READ/IDENTIFY/SET FEATURES) |
| `emulator/src/disk.rs` | new — flat disk image (load/blank/read_sector/write_sector) |
| `emulator/src/rtc.rs` | new — RTC-72421 stub with host/fixed/epoch policy stubs |
| `emulator/src/rom.rs` | new — dual 4 KiB ROM banks (base/video), Intel HEX parser |
| `emulator/src/peripherals.rs` | new — absent peripheral stubs (CH375, ESP, Multi-I/O kbd) |
| `emulator/src/cpu/mod.rs` | new — 6502 CPU state + reset vector; `step()` stub for WI-M1 |
| `emulator/src/cpu/flags.rs` | new — processor status register bit constants and helpers |
| `emulator/src/cpu/opcodes.rs` | new — `Opcode` enum stub + `decode()` placeholder |
| `emulator/tests/m1_serial_gate.rs` | new — M1 gate test placeholder |
| `emulator/tests/m2_mmu_gate.rs` | new — M2 gate test placeholder |
| `emulator/tests/m3_xt_ide_gate.rs` | new — M3 gate test placeholder |
| `emulator/tests/m4_dos_boot_gate.rs` | new — M4 gate test placeholder |
| `emulator/tests/m5_disk_io_gate.rs` | new — M5 gate test placeholder |
| `emulator/tests/m6_rtc_config_gate.rs` | new — M6 gate test placeholder |

## Verification

First verification — `cargo check` (initial build):

```
$ cd emulator && cargo check
   Compiling ... (dependencies)
    Checking emulator v0.1.0 (...)
warning: ... (26 dead-code warnings for scaffold stubs, expected)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 24.48s
```

Result: **PASS** — zero errors, 26 expected dead-code warnings for scaffold stubs that will be filled in by subsequent milestones.

Final proof — `cargo check` confirms no regressions after `.gitignore` addition and unstaging of `target/`:

```
$ cd emulator && cargo check
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.27s
```

Result: **PASS**

Commit: `git:8348ec3deef57d97af08ad874e65ff2b4855fb88`

## Remaining Risks

| ID | Status |
| --- | --- |
| REQ-WI-SETUP-01 | covered |
| REQ-WI-SETUP-02 | covered |
| REQ-WI-SETUP-03 | covered |
| REQ-WI-SETUP-04 | covered |
| REQ-WI-SETUP-05 | covered |

- `MmuPowerOnFill::Random` uses a deterministic `0xA5` fill rather than a PRNG-seeded random; this matches the spec intent (repeatable/deterministic) and the random fill contract for WI-M2 tests.
- `RtcPolicy::Host` does not yet populate BCD registers from host time — stubbed for WI-M6.
- `Cpu::step()` is a no-op stub — full opcode table implemented in WI-M1.
- All six gate tests are `#[ignore]`-marked; they require `rom.hex`/`disk.img` artifacts produced by later milestones.
- `config/default.toml` has `rtc_epoch` as a comment (`rtc_epoch` key) but the `Config` struct does not yet have a parsed field for it; WI-M6 will add the epoch field when the RTC clock-advance mode is implemented.
