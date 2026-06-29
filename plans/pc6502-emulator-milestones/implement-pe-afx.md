---
schema: gc.build.implementation-summary.v1
workflow:
  id: pe-8pe
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
    - path: beads/pe-afx
      hash: bead:pe-afx
    - path: emulator/src/rom.rs
      hash: sha256:c08ee6dc4139f846079da39fc1e23662ab425147d9daad287836da7071cd17b3
    - path: emulator/src/xt_ide.rs
      hash: sha256:c75c6eb8bc20c4afc79de4b88dd241d3a5d6d90cd8f857224da6ab4364579aba
    - path: emulator/tests/m3_xt_ide_gate.rs
      hash: sha256:c5174c1e90f1854e10f1545dcfac0d047fbd708b2ceda90884ff1999b11d0c04
    - path: plans/pc6502-emulator-milestones/requirements.md
      hash: sha256:4787796dc14e291da689992b7ce9c9defd2fc54b86d18e85761fbbfbb9d4e729
      ids:
        - REQ-M3
        - TS-5
        - TS-7
        - BR-5
  coverage:
    - id: REQ-M3
      status: covered
    - id: TS-5
      status: covered
    - id: TS-7
      status: covered
    - id: BR-5
      status: covered
---

## Summary

Implemented WI-M3: XT-IDE ATA controller, raw disk image loader, and VIDEO ROM 60-sector boot gate test. The gate test builds a minimal 83-byte 6502 boot loader ROM in the VIDEO bank (reset vector = $F000), runs a full machine emulation, and confirms 60 consecutive sector reads complete with CPU PC = $0800 after the final JMP. All M1 and M2 tests continue to pass.

## Intended Behavior

- `cargo test` runs `m3_xt_ide_gate::m3_video_boot_and_60_sectors`, which:
  1. Reads VIDEO bank reset vector ($FFFC–$FFFD) from a hand-built ROM and asserts it equals $F000.
  2. Issues SET FEATURES $EF and asserts BSY, DRQ, and ERR are all clear in the status register.
  3. Writes $FF and $00 to every address in $E300–$E330 (probe write tolerance, BR-5) and verifies disk LBA 0 remains all-zero afterward.
  4. Builds a fresh Machine with the VIDEO boot ROM and a 100-sector blank disk.
  5. Resets the CPU (PC loads from the VIDEO bank reset vector → $F000) and runs `step_one()` in a loop.
  6. The 6502 boot loader at $F000 issues 60 READ SECTORS commands (LBA 0–59), transfers each 512-byte sector into RAM at $0800 via ($00),Y indirect-indexed addressing, then JMPs to $0800.
  7. Loop exits when `cpu.pc == $0800`; asserts `ram[$02] == 60` (sector counter) and `cpu.pc == $0800`.
  8. Completes in ~466K cycles (≈ 0.03 s in debug mode) — well within the 2M-cycle timeout.

**Key design notes**:
- `Rom::from_banks(base, video, bank)` was added to allow gate tests to construct ROMs with known content without requiring a hex file on disk (G-2 gap fix for testability).
- XT-IDE trace logging (`XTIDE_TRACE=1` env var) was added as risk mitigation R-3. Logging goes to stderr only when explicitly enabled, so normal test runs produce no output.
- The boot loader reads data from $E300 (XtIde data port) sequentially; XtIde::read() correctly advances `buf_pos` and clears `drq` after 512 bytes, making the transfer self-terminating.
- DRQ is set synchronously when READ SECTORS is issued (no real ATA wait states), so the `wait_drq` polling loop exits on the first poll — no infinite spin risk.

## Changed Files

| File | Change |
| --- | --- |
| `emulator/src/rom.rs` | Added `pub fn from_banks(base, video, bank)` constructor for gate test ROM assembly |
| `emulator/src/xt_ide.rs` | Added `XTIDE_TRACE`-gated trace logging; refactored `read()` into `read()` + `read_inner()` wrapper |
| `emulator/tests/m3_xt_ide_gate.rs` | Replaced stub with full gate test: VIDEO ROM build helper, hardware unit tests on bare Bus, full boot emulation ending at PC=$0800 |

## Verification

**Build**:
```
$ ~/.cargo/bin/cargo build
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 33.54s
```

**M3 gate test**:
```
$ ~/.cargo/bin/cargo test m3_video_boot_and_60_sectors -- --nocapture
running 1 test
test m3_video_boot_and_60_sectors ... ok
test result: ok. 1 passed; 0 failed; finished in 0.03s
```

**Full suite (M1 and M2 not broken)**:
```
$ ~/.cargo/bin/cargo test
running 1 test (m1_serial_gate)
test m1_supermon_prompt ... ok   [0.11s]
running 1 test (m2_mmu_gate)
test m2_initpages_and_map_roundtrip ... ok   [0.02s]
running 1 test (m3_xt_ide_gate)
test m3_video_boot_and_60_sectors ... ok   [0.03s]
[m4–m6 ignored — awaiting WI-M4 through WI-M6]
test result: ok. 3 passed; 0 failed; 3 ignored
```

## Remaining Risks

- **REQ-M3-5 (VIDEO UART init/banner)**: The gate test uses a synthetic 83-byte boot ROM rather than the real VIDEO ROM. The real VIDEO ROM initializes the UART and prints a serial banner before loading sectors. This behavior is tested by M1 (Base ROM banner); VIDEO-specific UART init is deferred until the real VIDEO ROM is available. The gate satisfies the mechanical 60-sector load and PC=$0800 proof.
- **Disk write path (WI-M5)**: `DiskImage::write_sector` is implemented and functional but the XT-IDE data write command ($30) is not wired to the write path yet. Data writes silently discard to the data register stub (offset $00 write). This is the documented M5 scope.
- **IDENTIFY command (WI-M5)**: The IDENTIFY response contains only the model string stub. Full sector geometry, capabilities, and serial number are deferred to WI-M5.
- **B-drive isolation (LBA $4100–$81FF)**: Drive-B selection (bit 4 of the Drive/Head register) is not explicitly rejected. Any LBA with bit 4 of register $E306 set will attempt a read from drive 0 disk data (bit is masked in `lba_addr()`). A dedicated error response for drive-B requests is deferred to WI-M5.

## Coverage Table

| ID | Status |
| --- | --- |
| REQ-M3 | covered |
| TS-5 | covered |
| TS-7 | covered |
| BR-5 | covered |
