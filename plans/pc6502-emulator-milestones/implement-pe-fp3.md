---
schema: gc.build.implementation-summary.v1
workflow:
  id: pe-8na
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
    - path: beads/pe-fp3
      hash: bead:pe-fp3
    - path: emulator/src/mmu.rs
      hash: sha256:18390f245eb4275349fac5b80d07b00a9df323a954c25772e99c9af44b7e3dc0
    - path: emulator/tests/m2_mmu_gate.rs
      hash: sha256:6b2f0ba115308fe85be1f9f16e148c14f015c6a2866b3e6f41e2cd9da98c6951
    - path: emulator/src/bus.rs
      hash: sha256:9bf0c01c261d2e229ff387dd5757bf5a8e033b64bc29835c0d3804dc4393ce5e
    - path: emulator/src/config.rs
      hash: sha256:2ba3551f5a35ad4f3a517f126ec87028e8d6744dca641bf0dec6e26e23e36784
    - path: plans/pc6502-emulator-milestones/requirements.md
      hash: sha256:4787796dc14e291da689992b7ce9c9defd2fc54b86d18e85761fbbfbb9d4e729
      ids:
        - REQ-M2-1
        - REQ-M2-2
        - REQ-M2-3
        - REQ-M2-4
        - REQ-M2-5
        - REQ-M2-6
        - TS-3
        - BR-3
        - BR-4
        - BR-9
  coverage:
    - id: REQ-M2-1
      status: covered
    - id: REQ-M2-2
      status: covered
    - id: REQ-M2-3
      status: covered
    - id: REQ-M2-4
      status: covered
    - id: REQ-M2-5
      status: covered
    - id: REQ-M2-6
      status: covered
    - id: TS-3
      status: covered
    - id: BR-3
      status: covered
    - id: BR-4
      status: covered
    - id: BR-9
      status: covered
---

## Summary

Implemented WI-M2: 64-task MMU with correct `setup_task`/`active_task` separation and the M2 gate test. The gate test boots the ROM, verifies INITPAGES completes with the MMU enabled and task 0 active (`$EFE4 = 0x80`), confirms task-1 page $C→$10 ($C000→$10000) and page $D→$11 ($D000→$11000), exercises the task-0 edit-window round-trip (16-byte write/read via $EFD0–$EFDF), checks the banner persists in ACIA output, and verifies the task-mask clamp ($FF→$EFE0 gives $3F in $EFE4 bits 5:0). Both `m1_supermon_prompt` and `m2_initpages_and_map_roundtrip` pass.

## Intended Behavior

- `cargo test` runs `m2_mmu_gate::m2_initpages_and_map_roundtrip`, which:
  1. Boots the ROM until the Supermon register dump (`;`) appears, confirming INITPAGES has run.
  2. Asserts `$EFE4 & 0x80 == 0x80` (MMU enabled) and `$EFE4 & 0x3F == 0x00` (task 0 active).
  3. Asserts the ACIA output contains `_` (banner printed while MMU was active).
  4. Calls `mmu.translate(1, 0xC) == 0x10` and `mmu.translate(1, 0xD) == 0x11` to verify task-1's extended-RAM mapping; also asserts `mmu.translate(0, 0xC) == 0x0C` (task-0 identity).
  5. Writes 16 known bytes to $EFD0–$EFDF with setup task set to 0, reads them back identically.
  6. Writes $FF to $EFE0 and reads $EFE4 to confirm bits 5:0 = $3F (6-bit task mask).

**Key design note**: The MMU has two distinct task selectors — `active_task` ($EFE0, used for address translation) and `setup_task` ($EFE1, controls which task's page entries the edit window exposes). The previous scaffold incorrectly treated both as `active_task`. INITPAGES writes $EFE1 to switch the edit window between task 1 and task 0 while the MMU is disabled, so the buggy implementation happened to produce the correct map — but downstream DOS/65 far-call switching (M4) requires the distinction. The fix was applied in this milestone.

**Cherry-pick note**: The M1 CPU/ACIA/ROM implementation (commit `7bd8425`) was not merged to the main branch before the pe-fp3 worktree was created. It was cherry-picked onto the pe-fp3 branch as a prerequisite for running the ROM-based M2 gate test. The cherry-pick applied cleanly.

## Changed Files

| File | Change |
| --- | --- |
| `emulator/src/mmu.rs` | Added `setup_task: u8` field; $EFE0 write sets `active_task`, $EFE1 write sets `setup_task`; edit window reads/writes now index `setup_task`'s map rows; added `setup_task()` accessor; updated doc comments |
| `emulator/tests/m2_mmu_gate.rs` | Replaced stub with full gate test: boot to register dump, five acceptance assertions, no `#[ignore]` |

## Verification

**Build**:
```
$ cargo build --tests
   Compiling emulator v0.1.0 (…/worktrees/pe-fp3/emulator)
    Finished `test` profile [unoptimized + debuginfo] target(s)
```

**M2 gate test**:
```
$ cargo test m2_initpages -- --nocapture
running 1 test
test m2_initpages_and_map_roundtrip ... ok
test result: ok. 1 passed; 0 failed; finished in 0.08s
```

**Full suite (M1 not broken)**:
```
$ cargo test
running 1 test (m1_serial_gate)
test m1_supermon_prompt ... ok   [0.15s]
running 1 test (m2_mmu_gate)
test m2_initpages_and_map_roundtrip ... ok   [0.08s]
[m3–m6 ignored — awaiting WI-M3 through WI-M6]
test result: ok. 2 passed; 0 failed; 4 ignored
```

## Remaining Risks

- **setup_task vs. active_task during MMU-enabled execution**: The fix correctly decouples the edit-window task from the translation task. SETPAGE (used by DOS/65 M4) disables the MMU, sets $EFE1 to the target task, writes one edit-window entry, re-enables the MMU — this sequence is now handled correctly.
- **$EFE6 / $EFE7 stubs**: The ISA TC bit ($EFE6) and current I/O page ($EFE7) registers return open-bus. The BIOS does not read these in the boot path tested here, so the stubs are safe through M2.
- **io_rom_always**: Physical pages that overlap the I/O or ROM logical ranges are not forced to I/O/ROM decode when accessed through MMU translation. The bus decodes on logical addresses first, which matches the confirmed hardware behavior for tasks 0 and 1. The `io_rom_always` config knob (OQ-R0.7) is present but unimplemented; the identity-map behavior is correct for all milestones through M4.
- **Task 0 edit-window side effect**: The roundtrip test overwrites task 0's page map with the test pattern, leaving task 0 without a valid identity map for the remainder of the test. Subsequent assertions (task-mask check) use only I/O registers and do not involve RAM translation, so the corrupted map does not affect test correctness.

## Coverage Table

| ID | Status |
| --- | --- |
| REQ-M2-1 | covered |
| REQ-M2-2 | covered |
| REQ-M2-3 | covered |
| REQ-M2-4 | covered |
| REQ-M2-5 | covered |
| REQ-M2-6 | covered |
| TS-3 | covered |
| BR-3 | covered |
| BR-4 | covered |
| BR-9 | covered |
