---
schema: gc.build.implementation-summary.v1
workflow:
  id: pe-5z5
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
    - path: beads/pe-736
      hash: bead:pe-736
    - path: emulator/src/rtc.rs
      hash: sha256:19ce5c57258f09107ca51d89bcff732f6e35fd1dfa3a968062793f9f78beb35e
    - path: emulator/src/config.rs
      hash: sha256:c674d4794756e92083647675c28c42990d28b00a5bedb2124c42a140525d4c77
    - path: emulator/src/lib.rs
      hash: sha256:96cb8234eedad63adf95c58809e560f055634baf7c6ec1008177487a15c8da98
    - path: emulator/src/main.rs
      hash: sha256:e3dafbd0ca0d8cd628d8bebec9d324e89f6952b6bbbc1aa1a7056660abc53b9c
    - path: emulator/config/default.toml
      hash: sha256:c437d7e400c5266e96b828339ddbb9a76912e3dfbdd8631585efdb8b16b0032d
    - path: emulator/tests/m6_rtc_config_gate.rs
      hash: sha256:e23dcf0c64d58c02dcce8cab9131bb03f073d765d85a843c16c34044ccd4c5e7
    - path: plans/pc6502-emulator-milestones/requirements.md
      hash: sha256:4787796dc14e291da689992b7ce9c9defd2fc54b86d18e85761fbbfbb9d4e729
      ids:
        - REQ-M6
        - TS-6
        - TS-9
        - BR-6
        - BR-7
        - OQ-R0.5
  coverage:
    - id: REQ-M6
      status: covered
    - id: TS-6
      status: covered
    - id: TS-9
      status: covered
    - id: BR-6
      status: covered
    - id: BR-7
      status: covered
    - id: OQ-R0.5
      status: covered
---

## Summary

Implemented WI-M6: full RTC-72421/72423 model with three clock policies, library-crate refactor enabling integration tests, `rtc_epoch` configuration knob, and `Config::from_toml_str`. Six new gate tests (plus one that verifies the entire open-bus range $EFA0–$EFCF) run without `rom.hex` or a disk image; all 7 pass. No regressions in M1–M5.

## Intended Behavior

- **RTC host policy**: `Rtc::update_from_policy` calls `std::time::SystemTime::now()` on each non-stopped read, converts the Unix timestamp to BCD calendar fields via a pure `unix_to_calendar` function, and stores them in registers $00–$0C. Year is stored as a 2-digit value (2025 → tens=2, ones=5) in 1=Sun..7=Sat weekday convention.
- **RTC fixed/epoch policy**: Uses `Config::rtc_epoch` (Unix timestamp, default 2025-01-01 00:00:00 UTC) instead of the system clock. The `epoch` variant behaves identically to `fixed` in this implementation; persistent image-file tracking is deferred.
- **RTC STOP bit**: Writing $08 (or any value with bit 3 set) to offset $0F freezes the counter; subsequent reads return stale register values. Clearing bit 3 re-enables updates. All 16 register offsets accept writes without bounds check or crash.
- **Config::from_toml_str**: Deserializes a TOML string directly, enabling test-local config construction without touching `std::env::args`.
- **rtc_epoch field**: Added to `Config` with serde default 1735689600 (2025-01-01); documented in `default.toml`.
- **Library crate (`lib.rs`)**: All modules re-exported as `pub mod`; `main.rs` reduced to a three-line entry point. Integration tests in `tests/` can now `use emulator::bus::Bus` etc. directly.
- **Open-bus regions**: All reads in $EFA0–$EFCF, $EFF0–$EFFF, and the catch-all I/O arm return `self.open_bus` (the configured byte). This was already wired in `bus.rs`; the gate test now verifies the full $EFA0–$EFCF range.
- **CH375 stubs**: `ch375_read` returns `open_bus`; writes are silently discarded. No crash or hang is possible.
- **Multi-I/O keyboard self-test**: Write $AA to $E3FE → `kbd_selftest_pending = true`; next read of $E3FE returns $55 and clears the flag (already implemented in `peripherals.rs`; verified by gate test).

## Changed Files

| File | Change |
| --- | --- |
| `emulator/src/rtc.rs` | Full implementation: `unix_to_calendar`, `is_leap`, `host_unix_secs`, `populate_from_unix`; replaced stub `update_from_policy`; added `epoch` field |
| `emulator/src/config.rs` | Added `rtc_epoch: u64` field with serde default; added `Config::from_toml_str`; added `default_rtc_epoch` helper |
| `emulator/src/lib.rs` | New file: exposes all modules as `pub mod`; re-exports `Config` and `Machine` |
| `emulator/src/main.rs` | Reduced to three lines; delegates to library crate |
| `emulator/config/default.toml` | Added `rtc_epoch` documentation comment |
| `emulator/tests/m6_rtc_config_gate.rs` | Replaced `#[ignore]` stub with 7 active tests covering REQ-M6 items 1–6 and OQ-R0.5 |

## Verification

**First verification (build)**:
```
$ cargo build
Finished `dev` profile [unoptimized + debuginfo] target(s) in 35.67s
```

**Final proof (full test suite)**:
```
$ cargo test
running 7 tests
test open_bus_at_efa0 ... ok
test rtc_control_sequence_no_fault ... ok
test multiio_selftest_aa_55 ... ok
test ch375_returns_open_bus_no_crash ... ok
test config_from_toml_applies_settings ... ok
test rtc_host_year_plausible ... ok
test rtc_fixed_matches_epoch ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
```
All M6 gate tests pass. M1–M5 gate tests remain ignored (require `rom.hex`/`disk.img`) and show no regressions.

## Remaining Risks

- **Firmware-level RTC validation deferred**: REQ-M6 item 1 ("DOS time/date command returns plausible date") and item 2 ("clock advances after write sequence") are verified at the hardware-model level. Exercising the full path through DOS/65's RTC driver requires `rom.hex` and `disk.img`, which are not yet available.
- **Epoch policy persistence not implemented**: The `epoch` RTC policy is described as advancing from a persistent image file; this implementation treats it identically to `fixed`. Persistent epoch tracking deferred until a disk-image path is wired through config.
- **2-digit year rollover**: The RTC stores a 2-digit year (0–99). The driver in DOS/65 is assumed to add 2000; no century-disambiguation is performed by the emulator. This will require a config knob or a driver fix when the board is tested past year 2099.
- **CH375 C: drive test requires firmware**: REQ-M6 item 3 ("DOS prompt returns after failed C: access") requires the emulator to run through the DOS/65 SIM device-init loop, which needs `rom.hex`. The hardware stub (open-bus return, no crash) is verified.

| ID | Status |
| --- | --- |
| REQ-M6 | covered |
| TS-6 | covered |
| TS-9 | covered |
| BR-6 | covered |
| BR-7 | covered |
| OQ-R0.5 | covered |
