---
schema: gc.build.repair.v1
workflow:
  id: pe-w1b
  formula: build-from-requirements
producer:
  formula: fix-loop-base
  stage: repair-review
  bead: pe-yml
  attempt: 1
status: approved
---

## Repair Summary

Both critical findings from `review.md` are resolved. The consolidated branch
`pe-consolidated` was created from `pe-czj` (M1–M5 passing, correct two-register
MMU), the M6 commit cherry-picked on top, one compilation fix applied, and the
result merged to `main` at `0b1592f`.

## Findings Addressed

### F-1 — MMU setup_task/active_task conflation (CRITICAL)

**Resolution**: By building from `pe-czj` rather than the scaffold, the
two-register MMU from `pe-fp3` is already present. The consolidated `mmu.rs`
has both `active_task` (set by `$EFE0`) and `setup_task` (set by `$EFE1`),
with the edit window indexing `setup_task` and address translation using
`active_task`. `m2_initpages_and_map_roundtrip` passes.

### F-2 — M1–M5 gate tests scaffold-reverted (CRITICAL)

**Resolution**: Building from `pe-czj` restores all five full gate test
implementations. No tests are `#[ignore]`-marked in the consolidated branch.

## Additional Fix

A compile error arose from the cherry-pick: `Config`'s manual `Default` impl
(from pe-czj) was missing the new `rtc_epoch` field added by the M6 commit.
Added `rtc_epoch: default_rtc_epoch()` to close the gap without changing any
other field defaults.

## Verification

`cargo test` run from `main` (commit `0b1592f`) with
`PC6502_ROM_HEX=/mnt/fileserver/Vintage/Projects/PC6502Emulator/PC6502_firmware_source/rom.hex`:

| Test | Result |
|------|--------|
| m1_supermon_prompt | ok (0.15 s) |
| m2_initpages_and_map_roundtrip | ok (0.06 s) |
| m3_video_boot_and_60_sectors | ok (0.02 s) |
| m4_dos65_cold_boot_prompt | ok (0.00 s) |
| m5_disk_io_gate | ok (0.00 s) |
| rtc_host_year_plausible | ok |
| rtc_fixed_matches_epoch | ok |
| rtc_control_sequence_no_fault | ok |
| open_bus_at_efa0 | ok |
| multiio_selftest_aa_55 | ok |
| config_from_toml_applies_settings | ok |
| ch375_returns_open_bus_no_crash | ok |

13 passed, 0 failed, 0 ignored.

## Consolidated Branch

- Branch: `pe-consolidated` (merged to `main`)
- Merge commit: `0b1592f`
- Base: `pe-czj` (98b92f2)
- M6 cherry-pick: 23fb5a8 → 5b67515 (after conflict resolution)
- Compile fix: 2b6b14b
