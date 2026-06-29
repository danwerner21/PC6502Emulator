---
schema: gc.build.review.v1
workflow:
  id: pe-w1b
  formula: build-from-requirements
methodology:
  pack: gascity
  name: build-from-requirements
producer:
  formula: review
  stage: review
  attempt: 1
status: changes_required
trace:
  upstream:
    - path: plans/pc6502-emulator-milestones/requirements.md
      hash: sha256:4787796dc14e291da689992b7ce9c9defd2fc54b86d18e85761fbbfbb9d4e729
      ids:
        - REQ-M1
        - REQ-M2
        - REQ-M3
        - REQ-M4
        - REQ-M5
        - REQ-M6
    - path: plans/pc6502-emulator-milestones/implementation-summary.md
      hash: bead:pe-6op
    - path: plans/pc6502-emulator-milestones/implement-pe-ztw.md
      hash: bead:pe-ztw
    - path: plans/pc6502-emulator-milestones/implement-pe-lbw.md
      hash: bead:pe-lbw
    - path: plans/pc6502-emulator-milestones/implement-pe-fp3.md
      hash: bead:pe-fp3
    - path: plans/pc6502-emulator-milestones/implement-pe-afx.md
      hash: bead:pe-afx
    - path: plans/pc6502-emulator-milestones/implement-pe-cs9.md
      hash: bead:pe-cs9
    - path: plans/pc6502-emulator-milestones/implement-pe-czj.md
      hash: bead:pe-czj
    - path: plans/pc6502-emulator-milestones/implement-pe-736.md
      hash: bead:pe-736
    - path: emulator/src/mmu.rs
      hash: live
    - path: worktrees/pe-fp3/emulator/src/mmu.rs
      hash: live
    - path: worktrees/pe-736/emulator/src/mmu.rs
      hash: live
    - path: worktrees/pe-czj/emulator/tests/m5_disk_io_gate.rs
      hash: live
    - path: worktrees/pe-736/emulator/tests/m1_serial_gate.rs
      hash: live
  coverage:
    - id: REQ-M1
      status: covered
    - id: REQ-M2
      status: covered
    - id: REQ-M3
      status: deferred
    - id: REQ-M4
      status: deferred
    - id: REQ-M5
      status: deferred
    - id: REQ-M6
      status: covered
---

## Verdict

**`changes_required`**

The implementation work items pe-ztw through pe-czj implement and pass M1–M5
gate tests in a correct incremental chain. The M6 work item (pe-736) was
built from the scaffold rather than from pe-czj, causing two regressions that
must be repaired before the implementation can be approved:

1. **Critical — MMU setup_task regression** (`worktrees/pe-736/emulator/src/mmu.rs`):
   pe-736 reverts to the scaffold MMU, which conflates `active_task` and
   `setup_task`. Both `$EFE0` and `$EFE1` writes go to `active_task`; the
   edit window always targets `active_task` rather than `setup_task`. This
   breaks BIOS INITPAGES task-map population and DOS/65 far-call
   task-switching (REQ-M2, REQ-M4-3). pe-fp3 introduced the correct
   two-register design; pe-736 must adopt it.

2. **Critical — M1–M5 gate tests regressed to scaffold** (`worktrees/pe-736/emulator/tests/m{1..5}_*.rs`):
   All five earlier gate test files in pe-736 are `#[ignore]`-marked
   placeholders. The implemented tests (passing in pe-lbw through pe-czj) are
   absent. A consolidated branch built from pe-czj + pe-736 M6 additions must
   show all six milestone tests active and passing.

Secondary deferred items are accepted for this phase pending `disk.img`
availability; they do not require changes to the implementation code:

- REQ-M3-5: VIDEO ROM UART banner — synthetic boot ROM used; real VIDEO ROM
  UART path not exercised.
- REQ-M4-3: Far-call dispatcher — synthetic ROM approximates task-switching
  mechanics; real $FFF0/$C000 far-call stub not verified.
- REQ-M4-4: SIM cold-init loop — synthetic ROM skips device-probe sequence.
- REQ-M5-1,2,4: DIR listing, .COM load, drive-E failure — require disk.img.
- REQ-M6-1,3: RTC via DOS/65, CH375 C: drive — require disk.img.

## Findings

### F-1 · CRITICAL · MMU setup_task/active_task conflation in pe-736

**Location**: `worktrees/pe-736/emulator/src/mmu.rs` lines 17–115

**Observation**: `Mmu` struct has only `active_task: u8`; no `setup_task`
field. `io_write` arms `0x10 | 0x11` both write `self.active_task = val &
0x3F`. `io_read` edit-window arm uses `self.active_task` as the map row
selector.

**Expected** (per pe-fp3 `worktrees/pe-fp3/emulator/src/mmu.rs`): Separate
`active_task` (set by `$EFE0`) and `setup_task` (set by `$EFE1`). Edit window
reads/writes index `setup_task`; address translation uses `active_task`;
`$EFE4` status reports `active_task`.

**Impact**: BIOS INITPAGES writes `$EFE1` to redirect the edit window to
task-1 before populating its page entries C→$10 and D→$11. With the buggy
implementation the edit window always targets `active_task=0`, silently
overwriting task-0's map instead of task-1's. This would cause incorrect
address translation once the MMU is enabled and corrupted task-1 map pages.
REQ-M2, BR-3, BR-4, REQ-M4-3 all rely on correct `setup_task` behavior.

**Fix**: Port the two-field design from `worktrees/pe-fp3/emulator/src/mmu.rs`
into the consolidated branch. The diff is mechanical: add `setup_task: u8` to
the struct, initialize to 0, change edit-window arms to use `self.setup_task`,
split `0x10 | 0x11 =>` into separate arms for `0x10` (active_task) and `0x11`
(setup_task), add `pub fn setup_task(&self) -> u8` accessor.

**Gate test proof**: `m2_initpages_and_map_roundtrip` passes in pe-fp3, pe-afx,
pe-cs9, pe-czj. Once pe-736 sources the corrected MMU, this test must pass in
the consolidated branch as well.

---

### F-2 · CRITICAL · Gate tests M1–M5 regressed to scaffold in pe-736

**Location**: `worktrees/pe-736/emulator/tests/m{1,2,3,4,5}_*_gate.rs`

**Observation**: All five files contain only `#[ignore]`-marked stub functions.
The full test implementations (verified passing in pe-lbw through pe-czj) are
absent. Running `cargo test` in pe-736 shows 5 ignored gate tests and 7
passing M6 unit tests; M1–M5 gates are not exercised.

**Expected**: All six gate tests active and passing in the final consolidated
state (`m1_supermon_prompt`, `m2_initpages_and_map_roundtrip`,
`m3_video_boot_and_60_sectors`, `m4_dos65_cold_boot_prompt`,
`m5_disk_io_gate`, plus the 7 M6 unit tests).

**Root cause**: pe-736 was branched from the scaffold (pe-ztw/main) rather
than from pe-czj. The M1–M5 test implementations never landed in pe-736's
working tree.

**Fix**: Build the consolidated branch starting from pe-czj. Cherry-pick or
apply pe-736's M6-specific changes on top:
- `emulator/src/rtc.rs` (full RTC implementation)
- `emulator/src/config.rs` (rtc_epoch field, from_toml_str)
- `emulator/src/lib.rs` (pub mod re-exports, library crate)
- `emulator/src/main.rs` (three-line entry point)
- `emulator/config/default.toml` (rtc_epoch doc)
- `emulator/tests/m6_rtc_config_gate.rs` (7 new gate tests)

The corrected MMU (F-1) must also be included — take it from pe-fp3 rather
than pe-736.

---

### F-3 · DEFERRED · REQ-M3-5 VIDEO ROM UART init and banner

**Location**: `worktrees/pe-afx/emulator/tests/m3_xt_ide_gate.rs`

**Observation**: M3 gate uses a synthetic 83-byte boot ROM, not the real VIDEO
ROM. VIDEO-specific UART initialization and serial banner output are not
exercised; only the 60-sector load and PC=$0800 assertion are verified.

**Acceptance criteria affected**: REQ-M3-5 ("Serial banner appears — VIDEO bank
also initializes UART").

**Disposition**: Deferred pending real VIDEO ROM and disk.img. The real VIDEO
ROM is present at `PC6502_firmware_source/` as `6502PCbiosvideo.lst` (listing
only; no prebuilt `.hex`). Until a VIDEO bank `rom.hex` is available, this
criterion cannot be gate-tested. No code change required now.

---

### F-4 · DEFERRED · REQ-M4-3 Far-call dispatcher verification

**Location**: `worktrees/pe-cs9/emulator/tests/m4_dos_boot_gate.rs`

**Observation**: The synthetic ROM in the M4 gate test performs direct
task-switch mechanics but does not implement the actual far-call stub at `$FFF0`
or the task-1 dispatcher at `$C000` that real DOS/65 firmware uses.
REQ-M4-3 ("Far call `$FFF0` → dispatcher `$C000` in task 1 → returns to
task-0 caller with A preserved") is not covered by an executable gate.

**Disposition**: Deferred pending `disk.img` with real DOS/65 image. The
infrastructure (task-switch registers, MMU, bus) is proven correct by M2 and
M4 gate tests. No code change required now.

---

### F-5 · DEFERRED · REQ-M5-1,2,4 DIR listing, .COM load, drive-E failure

**Location**: `worktrees/pe-czj/emulator/tests/m5_disk_io_gate.rs`

**Observation**: M5 gate tests the XT-IDE controller at the bus level and via
synthetic CPU ROM. REQ-M5-1 (DIR A: listing from CP/M filesystem),
REQ-M5-2 (.COM load and execution), and REQ-M5-4 (drive E failure via DOS/65
SIM) require a real CP/M disk image and the DOS/65 SIM layer.

**Disposition**: Deferred pending `disk.img`. The controller write/read
mechanics are verified. No code change required now.

---

### F-6 · DEFERRED · REQ-M6-1,3 Firmware-level RTC and CH375 C: drive

**Location**: `worktrees/pe-736/emulator/tests/m6_rtc_config_gate.rs`

**Observation**: RTC read returning a plausible date (REQ-M6-1) and CH375
absent-device behavior causing a safe DOS return from C: access (REQ-M6-3) are
verified at the hardware-model and stub level only. The DOS/65 RTC driver call
path and SIM device-probe loop are not exercised without a real disk image.

**Disposition**: Deferred pending `disk.img`. No code change required now.

## Verification

### Test execution results (per worktree)

| Worktree | Gate tests active | Status |
|----------|-------------------|--------|
| pe-lbw (M1) | m1_supermon_prompt | PASS (0.12 s) |
| pe-fp3 (M2) | m1, m2 | PASS (0.12 s + 0.14 s) |
| pe-afx (M3) | m1, m2, m3 | PASS |
| pe-cs9 (M4) | m1, m2, m3, m4 | PASS |
| pe-czj (M5) | m1, m2, m3, m4, m5 | PASS |
| pe-736 (M6) | M1–M5 IGNORED (scaffold); 7 M6 unit tests | PASS for M6 only |

M1 test uses real `rom.hex` at `PC6502_firmware_source/rom.hex` (19 KiB, dated
Jun 7 2025); it boots through BIOS delay, ACIA init, banner, BRK→Supermon,
register dump, `> F000` round-trip, and `G F000` re-boot in 0.12 s.

`disk.img` is not present; M3–M6 firmware-level gate tests (those requiring
disk I/O via real DOS/65) remain deferred.

### Fix handoff

The repair stage (`pe-yml` / `fix-loop-base`) must produce a consolidated
branch that:

1. Starts from `worktrees/pe-czj` (all M1–M5 tests passing, correct MMU).
2. Applies pe-736 M6 additions (rtc.rs, config.rs, lib.rs, main.rs,
   default.toml, m6_rtc_config_gate.rs).
3. Confirms the MMU uses the two-register design from pe-fp3 (not the scaffold
   in pe-736).
4. Passes `cargo test` with: m1 ok, m2 ok, m3 ok, m4 ok, m5_disk_io_gate ok,
   7 M6 unit tests ok — all in a single test run on the consolidated branch.
5. Merges the consolidated result to `main`.

### Coverage

| Requirement | Status | Notes |
|-------------|--------|-------|
| REQ-M1 (1–6) | covered | M1 gate passes with real rom.hex |
| REQ-M2 (1–6) | covered | M2 gate passes in pe-fp3+ |
| REQ-M3-1 | covered | VIDEO reset vector unit test |
| REQ-M3-2 | covered | BR-5 probe write unit test |
| REQ-M3-3 | covered | SET FEATURES unit test |
| REQ-M3-4 | covered | 60-sector M3 gate |
| REQ-M3-5 | deferred | VIDEO ROM UART banner; awaiting VIDEO rom.hex |
| REQ-M3-6 | covered | PC=$0800 assertion in M3 gate |
| REQ-M4-1 | covered | Synthetic M4 gate — task-0 copy to $B800 |
| REQ-M4-2 | covered | Synthetic M4 gate — task-1 physical $10000 |
| REQ-M4-3 | deferred | Far-call stub; awaiting disk.img |
| REQ-M4-4 | deferred | SIM cold-init loop; awaiting disk.img |
| REQ-M4-5 | covered | DOS/65 banner and A> in synthetic M4 gate |
| REQ-M4-6 | covered | Echo A:\r in synthetic M4 gate |
| REQ-M5-1 | deferred | DIR A: listing; awaiting disk.img |
| REQ-M5-2 | deferred | .COM load; awaiting disk.img |
| REQ-M5-3 | covered | Write–read roundtrip at controller and CPU level |
| REQ-M5-4 | deferred | Drive E failure; awaiting disk.img |
| REQ-M5-5 | covered | Bad-sector inject/clear API + unit test |
| REQ-M5-6 | covered | Drive B LBA isolation unit test |
| REQ-M6-1 | deferred | RTC via DOS/65 time/date; awaiting disk.img |
| REQ-M6-2 | covered | RTC write sequence unit test |
| REQ-M6-3 | deferred | CH375 C: via DOS/65; awaiting disk.img |
| REQ-M6-4 | covered | Multi-I/O keyboard $AA/$55 unit test |
| REQ-M6-5 | covered | Open-bus $EFA0–$EFCF unit test |
| REQ-M6-6 | covered | Config-from-TOML unit test |
| TS-1 | covered | Full 6502 opcode table; rom.hex boot |
| TS-2 | covered | 20-bit physical space, I/O overlay, ROM bank |
| TS-3 | covered | 64-task MMU — requires F-1 fix before consolidated |
| TS-4 | covered | ACIA TX/RX, TDRE, RDRF, programmed reset |
| TS-5 | covered | XT-IDE READ/WRITE/IDENTIFY/SET FEATURES |
| TS-6 | covered | RTC-72421 host/fixed/epoch, STOP bit |
| TS-7 | covered | rom.hex load, K1 bank selection, sector transfer |
| TS-8 | covered | CH375, ESP, Multi-I/O absent-device stubs |
| TS-9 | covered | All P0 unknowns as config knobs |
| BR-1 | covered | Reset vector $FFFC/$FFFD → $F000 |
| BR-2 | covered | ACIA init sequence, no crash |
| BR-3 | covered | Task-mask $FF→$EFE0 gives $3F in $EFE4[5:0] |
| BR-4 | covered | $EFE2=1 activates MMU translation |
| BR-5 | covered | XT-IDE probe write tolerance |
| BR-6 | covered | Open-bus reads return configured byte |
| BR-7 | covered | Absent-device stubs, no crash |
| BR-8 | covered | ROM write protection |
| BR-9 | covered | Physical holes >$7F return open-bus |
| OQ-R0.1 | covered | cpu_subtype config knob |
| OQ-R0.2 | covered | cpu_hz config knob |
| OQ-R0.3 | covered | mmu_power_on_fill config knob |
| OQ-R0.4 | covered | rom_bank config knob |
| OQ-R0.5 | covered | open_bus config knob |
| OQ-R0.6 | covered | shadow_addr_low config knob |
| OQ-R0.7 | covered | io_rom_always config knob |
| OQ-R1.1 | covered | acia_variant config knob |
| OQ-R1.4 | covered | acia_cts_default config knob |

### Unresolved findings

| ID | Severity | Description |
|----|----------|-------------|
| F-1 | CRITICAL | MMU setup_task absent in pe-736; must be ported from pe-fp3 |
| F-2 | CRITICAL | M1–M5 gate tests scaffold-reverted in pe-736; must consolidate from pe-czj |
| F-3 | DEFERRED | REQ-M3-5 VIDEO UART banner; awaiting VIDEO rom.hex |
| F-4 | DEFERRED | REQ-M4-3 far-call dispatcher; awaiting disk.img |
| F-5 | DEFERRED | REQ-M5-1,2,4 DIR/COM/drive-E; awaiting disk.img |
| F-6 | DEFERRED | REQ-M6-1,3 RTC + CH375 via DOS/65; awaiting disk.img |

Fix-attempt count on workflow root at time of this review: 0 (first review pass).
