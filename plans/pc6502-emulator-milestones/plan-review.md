---
schema: gc.build.plan-review.v1
workflow:
  id: pe-w1b
  formula: build-from-requirements
methodology:
  pack: gascity
  name: build-from-requirements
reviewer:
  bead_id: pe-n35
  role: review-synthesizer
  interaction_mode: autonomous
verdict: approved
reviewed_artifact:
  path: plans/pc6502-emulator-milestones/plan.md
  hash: "sha256:65264675202c8f4bf6654abc8c4d95799fa63e45ca3ed3428c7d31e2c7262b60"
requirements_artifact:
  path: plans/pc6502-emulator-milestones/requirements.md
  hash: "sha256:4787796dc14e291da689992b7ce9c9defd2fc54b86d18e85761fbbfbb9d4e729"
---

# Plan Review: PC6502 Emulator — Six-Milestone Build

**Verdict:** approved

**Reviewed:** 2026-06-28  
**Bead:** pe-n35  
**Plan bead:** pe-zfg  

---

## Summary

The implementation plan is well-structured and sufficiently detailed to proceed to
decomposition. All six milestone requirements (REQ-M1 through REQ-M6) are traced to
specific modules and acceptance criteria. Verification relies on running the actual
`rom.hex` and DOS/65 disk images rather than mocks, satisfying the gate-observable
requirement. Technology-stack selection (Rust, Cargo workspace) is well-justified.
Risks and assumptions are explicitly documented with mitigations. Two minor gaps are
noted below but neither blocks implementation.

---

## Coverage Analysis

### Requirements traceability

All six milestones are covered:

| Req ID | Plan coverage | Module(s) | Gate observable |
|--------|--------------|-----------|-----------------|
| REQ-M1 | ✓ | cpu/, bus.rs, acia.rs, rom.rs | `>` on serial after reset |
| REQ-M2 | ✓ | mmu.rs, bus.rs | INITPAGES complete; `$EFE4` readback |
| REQ-M3 | ✓ | xt_ide.rs, disk.rs | 60 sectors; PC=`$0800` |
| REQ-M4 | ✓ | bus.rs task-switch, peripherals.rs | `DOS/65` and `A>` on serial |
| REQ-M5 | ✓ | xt_ide.rs write path, disk.rs | `DIR A:` lists entries |
| REQ-M6 | ✓ | rtc.rs, config.rs | RTC date; config-file boot |

### Technical story coverage

| TS | Status | Notes |
|----|--------|-------|
| TS-1 (CPU opcodes + interrupts) | ✓ covered | cpu/mod.rs: all opcodes, NMOS/65C02 subtype |
| TS-2 (20-bit physical address space) | ✓ covered | bus.rs + mmu.rs translate logical → physical; 512 KiB SRAM implied |
| TS-3 (64-task MMU) | ✓ covered | mmu.rs: map store, edit window, control registers |
| TS-4 (ACIA 6551) | ✓ covered | acia.rs: TX/RX, TDRE, programmed reset |
| TS-5 (XT-IDE) | ✓ covered | xt_ide.rs + disk.rs: BSY/DRQ, read/write commands |
| TS-6 (RTC-72421/72423) | ✓ covered | rtc.rs: nibble registers, three clock policies |
| TS-7 (ROM + disk image load) | ✓ covered | rom.rs: Intel HEX parse, bank select; disk.rs: raw image |
| TS-8 (absent-device stubs) | ✓ covered | peripherals.rs: CH375, ESP, Multi-I/O |
| TS-9 (P0 unknowns as config knobs) | ✓ covered | All OQ-R0.x knobs present; see gap G-1 for OQ-R1.1 |

### Behavior requirement coverage

All BR-1 through BR-9 have explicit module assignments and verification steps.

### Open question resolution (config knobs)

| OQ ref | Config key | Status |
|--------|-----------|--------|
| OQ-R0.1 | `cpu_subtype` | ✓ |
| OQ-R0.2 | `cpu_hz` | ✓ |
| OQ-R0.3 | `mmu_power_on_fill` | ✓ |
| OQ-R0.4 | `rom_bank` | ✓ |
| OQ-R0.5 | `open_bus` | ✓ |
| OQ-R0.6 | `shadow_addr` | ✓ |
| OQ-R0.7 | `io_rom_always` | ✓ |
| OQ-R1.1 | *(missing — see G-1)* | ⚠ minor gap |
| OQ-R1.4 | `acia_cts_default` | ✓ |

---

## Findings

### G-1 (minor): OQ-R1.1 ACIA variant not in config knobs table

The requirements state OQ-R1.1 must be resolved as a named config knob
("expose ACIA variant as a configurable option"). The plan's config.rs table
omits an `acia_variant` (or equivalent) key. The ACIA section in the plan
models the confirmed polling contract, and the W65C51N transmit-ready errata is
explicitly deferred to the Non-Goals section. The omission is consistent with
the deferral decision. Implementers should add `acia_variant` to `default.toml`
(e.g., `acia_variant = "w65c51n"`) with a comment referencing OQ-R1.1 to
satisfy the requirement that the knob exist, even if the errata behavior is
not yet modeled.

**Severity:** minor — does not block decomposition  
**Action:** add `acia_variant` knob to config.rs and default.toml during M1 implementation

### G-2 (minor): `disk_image` config key referenced but not in table

M3 implementation step 1 references a `disk_image` config key for the raw
disk image path, but this key does not appear in the config.rs table. This is
an oversight in the plan document; the key must exist for XT-IDE to load the
image file.

**Severity:** minor — implementation-time addition is trivial  
**Action:** add `disk_image` (file path string) to config.rs table and default.toml during M3 implementation

---

## Structural Quality

**Assumptions:** A-1 through A-7 are explicit and consistent with the specification
documents. Each handles a real uncertainty (ROM bank byte ordering, NMOS default,
ATA sector unit, disk layout, TDRE always-set, far-call stub in ROM, no IRQ required).

**Risks:** R-1 through R-6 are identified at appropriate severity levels. R-1
(ROM bank layout) and R-3 (XT-IDE register spacing) are the two most likely to
block a milestone gate and each has a concrete mitigation (config knob + trace
logging).

**Milestone build order:** Sequential capability addition (CPU → MMU → XT-IDE →
DOS/65 boot → disk I/O → RTC/config) is correct. Each milestone's gate observable
is unambiguous and driven by the real firmware.

**Non-goals:** Explicit list matches requirements out-of-scope section. No missing
exclusions detected.

**Verification:** Per-milestone integration tests in `tests/mN_*_gate.rs` with a
regression policy (prior gates must stay green) is the right structure for
incremental firmware validation.

---

## Recommendation

Proceed to decomposition. The two minor gaps (G-1, G-2) should be noted in the
implementation work items for M1 and M3 respectively but do not require plan
revision before decomposition begins.
