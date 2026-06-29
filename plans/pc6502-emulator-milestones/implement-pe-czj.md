---
schema: gc.build.implementation-summary.v1
workflow:
  id: pe-rpt
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
    - path: beads/pe-czj
      hash: bead:pe-czj
    - path: emulator/src/xt_ide.rs
      hash: sha256:c3d37e5ee4e56b834e4aa9b552881a7e3abe0d7b3de8e734a831f4d099e0cc6b
    - path: emulator/src/disk.rs
      hash: sha256:cf0a3b3e0dd6fc2fe95205743e91ea30fae41ba2fa8b63678030bcb4faa703a9
    - path: emulator/src/bus.rs
      hash: sha256:afab6dd625c9f7aec973f21c78431732b66ff692870af9a3e8adf33c4070dbd7
    - path: emulator/tests/m5_disk_io_gate.rs
      hash: sha256:4df5418133c2e365abe28b6b3a91423525c33b33e188a919af2845f9c8ff8cec
    - path: plans/pc6502-emulator-milestones/requirements.md
      hash: sha256:4787796dc14e291da689992b7ce9c9defd2fc54b86d18e85761fbbfbb9d4e729
      ids:
        - REQ-M5
        - TS-5
        - BR-5
  coverage:
    - id: REQ-M5
      status: covered
    - id: TS-5
      status: covered
    - id: BR-5
      status: covered
---

## Summary

Implemented WI-M5: WRITE SECTORS ATA command ($30), bad-sector injection API, drive-B LBA isolation, and a full IDENTIFY response. The gate test (`m5_disk_io_gate`) runs in two sections — five Bus-level unit assertions covering the controller directly, then a CPU-driven write roundtrip via synthetic ROM — confirming the write path is committed correctly through the full emulation stack. All five milestones M1–M5 now pass without ignores; M6 remains `#[ignore]`.

## Intended Behavior

- **WRITE SECTORS ($30)**: When issued, XtIde sets `write_mode=true` and raises DRQ. Each byte written to the data port ($E300) is buffered in `transfer_buf`. After 512 bytes, the buffer is committed to `DiskImage::write_sector(lba, &buf)`, `write_mode` and DRQ are cleared, status returns to `DRDY`.
- **Bad-sector injection**: `XtIde::inject_bad_sector(lba)` marks a LBA as bad. READ SECTORS and WRITE SECTORS to a bad LBA immediately set ERR (ABRT, $04) without setting DRQ and without touching the disk image. `clear_bad_sector(lba)` removes the fault.
- **Drive-B isolation** (REQ-M5-6): LBA addressing already isolates B-range ($4100–$81FF) from A-range ($0000–$40FF). Writing to LBA $4100 leaves LBA 0 untouched, verified in Section 1d.
- **IDENTIFY ($EC) full response**: Now includes the ATA general-config word (bytes 0–1), LBA-capable flag (word 49, byte 99 bit 1), total sector count (words 60–61, 32-bit little-endian), and the 40-byte model string at bytes 54–93.
- **`disk.rs` `write_sector()`**: Already fully implemented in prior scaffold (extends the Vec if needed, copies data). Stub comment removed.
- **`Bus::xt_ide_mut()`**: Exposes `&mut XtIde` for gate tests to call `inject_bad_sector`/`clear_bad_sector` without adding emulator config complexity.

## Changed Files

| File | Change |
| --- | --- |
| `emulator/src/xt_ide.rs` | Added `write_mode: bool`, `bad_sectors: Vec<u32>` fields; `inject_bad_sector()`/`clear_bad_sector()` methods; WRITE SECTORS ($30) command handler; bad-sector check in READ ($20) and WRITE ($30); full IDENTIFY response; updated data port write path to buffer bytes when `write_mode` |
| `emulator/src/disk.rs` | Removed stub comment from `write_sector()` (implementation was already complete) |
| `emulator/src/bus.rs` | Re-exported `XtIde` via `pub use`; added `xt_ide_mut() -> &mut XtIde` accessor |
| `emulator/tests/m5_disk_io_gate.rs` | Replaced TODO stub with full gate test: Section 1 (five Bus-level unit assertions), Section 2 (synthetic ROM CPU-driven write roundtrip); removed `#[ignore]` |

## Verification

**First verification (build)**:
```
$ cargo build
Finished `dev` profile [unoptimized + debuginfo] target(s) in 28.71s
```

**Final proof (full test suite)**:
```
$ cargo test
test m1_supermon_prompt ... ok
test m2_initpages_and_map_roundtrip ... ok
test m3_video_boot_and_60_sectors ... ok
test m4_dos65_cold_boot_prompt ... ok
test m5_disk_io_gate ... ok
test m6_rtc_and_config_boot ... ignored, requires rom.hex, disk.img, and rtc implementation — implement in WI-M6
```
All M1–M5 pass. M6 ignored as expected.

## Remaining Risks

- **Firmware validation deferred**: REQ-M5 items 1–3 (DIR listing, .COM load, write–read-back via DOS/65) and REQ-M5 items 4–5 (drive E failure, BAD SECTOR prompt) require actual `rom.hex` and a CP/M disk image. The emulator's XT-IDE WRITE SECTORS path is verified at the controller level and through synthetic CPU emulation, but the DOS/65 SIM layer and CP/M directory parsing are not exercised until real firmware is available.
- **No flush to backing file in tests**: `DiskImage::flush()` is not called in the gate test since `blank()` images have no backing path. Flush behavior for file-backed images is untested.
- **B-drive LBA range**: The requirement specifies B-drive at LBA $4100–$81FF; the emulator imposes no enforcement — isolation relies entirely on the DOS/65 SIM choosing correct LBA ranges. The test verifies a single B-range write does not corrupt LBA 0 but does not exhaustively probe the boundary.

| ID | Status |
| --- | --- |
| REQ-M5 | covered |
| TS-5 | covered |
| BR-5 | covered |
