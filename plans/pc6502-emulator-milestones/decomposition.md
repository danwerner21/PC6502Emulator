---
schema: gc.build.decomposition.v1
workflow:
  id: pe-w1b
  formula: build-from-requirements
methodology:
  pack: gascity
  name: decomposition-base
producer:
  formula: decomposition-base
  stage: decompose
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
    - path: plans/pc6502-emulator-milestones/plan.md
      hash: "sha256:65264675202c8f4bf6654abc8c4d95799fa63e45ca3ed3428c7d31e2c7262b60"
    - path: plans/pc6502-emulator-milestones/plan-review.md
      hash: "sha256:740aa1ffa1143c44a73b80ba15e4cab4df5313bbe2209609841bac33eba91fe3"
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

# Decomposition: PC6502 Emulator — Six-Milestone Build

## Summary

The approved implementation plan is decomposed into seven sequential work items:
one scaffold bead (WI-SETUP) plus one bead per milestone (WI-M1 through WI-M6).
Each work item corresponds to the modules and gate observable defined in the
plan; each is independently verifiable by running the actual `rom.hex` and
DOS/65 disk images. No parallelism is used because each milestone builds
directly on the previous one's running emulator state.

Plan-review gaps incorporated as implementation notes:
- **G-1** (OQ-R1.1 `acia_variant` knob missing): added to WI-M1 scope.
- **G-2** (`disk_image` config key missing from plan table): added to WI-M3
  scope; placeholder key added in WI-M1 `config.rs`.

Unresolved ambiguities: none. All P0/P1 open questions (OQ-R0.1 through
OQ-R1.4) are resolved as named configuration knobs in `config.rs` and
`config/default.toml`. No hardware evidence is needed before starting
implementation.

Skipped work: none. All six milestone requirements are covered.

Blocked work: none. The plan is approved; no external dependency blocks
decomposition.

## Selected Downstream Formulas

| Formula | Purpose |
|---------|---------|
| `do-work-item` | Execute each WI-* implementation bead |
| `review` | Code review of completed milestone increments |

Each WI-* bead is a runnable implementation task. Downstream implementation
workers drain this convoy sequentially, respecting the `blocks` dependencies
between milestones.

## Implementation Convoy

Convoy: **pe-cfn** — *PC6502 Emulator — Implementation Convoy (M1–M6)*

| ID | Title | Depends On |
|----|-------|------------|
| pe-ztw | WI-SETUP: Cargo workspace scaffold | — |
| pe-lbw | WI-M1: CPU core, flat bus, ACIA, ROM loader | pe-ztw |
| pe-fp3 | WI-M2: MMU — 64-task map, address translation | pe-lbw |
| pe-afx | WI-M3: XT-IDE controller, disk image, VIDEO ROM boot | pe-fp3 |
| pe-cs9 | WI-M4: DOS/65 cold boot, absent-device stubs | pe-afx |
| pe-czj | WI-M5: DOS/65 disk read/write, DIR listing | pe-cs9 |
| pe-736 | WI-M6: RTC model, config-file boot, hardening | pe-czj |

This convoy is the implementation convoy only. It is distinct from the original
launch convoy (pe-v4x) and the workflow-control beads.

## Work Items

### WI-SETUP — Cargo workspace scaffold (pe-ztw)

**Req traceability:** All REQ-M1–M6 (structural prerequisite)  
**Plan section:** Repository layout  
**Expected files:**

- `emulator/Cargo.toml` — workspace root; `emulator` binary crate
- `emulator/Cargo.lock` — committed for reproducible builds
- `emulator/config/default.toml` — all P0/P1 knobs documented with OQ references
- `emulator/src/main.rs`, `config.rs`, `emulator.rs`, `bus.rs`, `acia.rs`,
  `mmu.rs`, `xt_ide.rs`, `disk.rs`, `rtc.rs`, `rom.rs`, `peripherals.rs`,
  `cpu/mod.rs`, `cpu/opcodes.rs`, `cpu/flags.rs` — stub skeletons
- `emulator/tests/` — placeholder gate test files

**Verification:** `cargo check` passes on empty stubs with no warnings.

**Dependencies:** none  
**Skipped:** none  
**Blocked:** none

---

### WI-M1 — CPU core, flat bus, ACIA, ROM loader (pe-lbw)

**Req traceability:** REQ-M1, TS-1, TS-2, TS-4, TS-7, TS-9, BR-1, BR-2, BR-6, BR-8  
**Open questions resolved:** OQ-R0.1 (`cpu_subtype`), OQ-R0.2 (`cpu_hz`),
OQ-R0.3 (`mmu_power_on_fill`), OQ-R0.4 (`rom_bank`), OQ-R0.5 (`open_bus`),
OQ-R0.6 (`shadow_addr`), OQ-R0.7 (`io_rom_always`), OQ-R1.1 (`acia_variant`),
OQ-R1.4 (`acia_cts_default`)  
**Plan gaps fixed:** G-1 (`acia_variant` knob added), G-2 (`disk_image` placeholder added)  
**Expected files:**

- `emulator/src/config.rs` — `Config` struct with all P0/P1 knobs; TOML load from `--config`
- `emulator/src/rom.rs` — `load_hex()`: parse Intel HEX, split Base/VIDEO 4 KiB banks; select via `rom_bank`
- `emulator/src/cpu/mod.rs` — `Cpu` struct, `step()`, RESET/NMI/IRQ/BRK dispatch; NMOS/65C02 subtype
- `emulator/src/cpu/opcodes.rs` — all documented 6502 opcodes; correct flags and address-mode wrapping
- `emulator/src/cpu/flags.rs` — flag register helpers (N, V, B, D, I, Z, C)
- `emulator/src/bus.rs` — flat 64 KiB: RAM `$0000–$DFFF`, I/O `$E000–$EFFF`, ROM `$F000–$FFFF`;
  open-bus for unmapped reads; ROM write protection
- `emulator/src/acia.rs` — 6551 at `$EF84–$EF87`; TX→stdout; TDRE always set;
  programmed reset (`$00`→`$EF85`); `acia_cts_default` and `acia_variant` knobs
- `emulator/src/emulator.rs` — `Machine` struct owning CPU, Bus, peripherals
- `emulator/src/main.rs` — CLI entry: `--config`, load ROM, emulator loop, stdout/stdin wiring
- `emulator/tests/m1_serial_gate.rs` — run until `>` in stdout ≤ 10 M cycles;
  inject `G F000\r`; assert banner re-emits without hang

**Gate observable:** Supermon `>` prompt on stdout after reset.

**Verification expectations:**
1. Reset vector fetch at `$FFFC–$FFFD` returns `$00 $F0` (Base bank).
2. SEI/CLD/LDX `$FF`/TXS startup sequence completes without CPU fault.
3. ACIA init `$00`→`$EF85`, `$0B`→`$EF86`, `$1E`→`$EF87` does not crash; TDRE bit 4 set.
4. Banner characters appear on stdout (TDRE polling exits normally).
5. BRK reaches Supermon; `>` received on stdout.
6. `G F000` re-runs reset path and re-emits banner.

**Dependencies:** pe-ztw  
**Skipped:** cycle-accurate timing, IRQ/NMI delivery (A-7), W65C51N errata  
**Blocked:** none

---

### WI-M2 — MMU: 64-task map, address translation (pe-fp3)

**Req traceability:** REQ-M2, TS-3, BR-3, BR-4, BR-9  
**Expected files:**

- `emulator/src/mmu.rs` — `Mmu` struct: 1024-byte map store (64 tasks × 16 bytes);
  `translate(logical_page, task) → physical_page`; edit window `$EFD0–$EFDF`;
  control registers `$EFE0–$EFE7`; `mmu_power_on_fill` from config;
  task mask (`$FF` → `$3F`); task-0 alias option
- `emulator/src/bus.rs` — MMU translation path inserted between logical and physical
  when enabled; `io_rom_always` config controls I/O/ROM decode precedence
- `emulator/tests/m2_mmu_gate.rs` — INITPAGES pass; `$EFE4` bit 7 set, task 0;
  edit-window round-trip; task-1 `$C000` → physical `$10000`; banner persists;
  `$FF`→`$EFE0` → `$EFE4` bits 5:0 = `$3F`

**Gate observable:** INITPAGES completes; `$EFE4` shows task 0 + enable bit set; banner still prints.

**Verification expectations:**
1. After BIOS INITPAGES: `read($EFE4) & 0xBF == 0x80` (enable bit set, task 0).
2. Write 16 known bytes to `$EFD0–$EFDF` task-0 edit window; read back identical.
3. Task-1 logical `$C000` translates to physical `$10000` (not `$0C000`).
4. Task-0 identity map: CPU `$1234` → physical `$01234`.
5. Banner still printed after INITPAGES (no hang).
6. Write `$FF` to `$EFE0`; `read($EFE4) & 0x3F == 0x3F`.

**Risk:** R-2 (indeterminate power-on map) — mitigated by `mmu_power_on_fill` knob.

**Dependencies:** pe-lbw  
**Skipped:** ISA TC bit semantics (stub returns 0 per spec note)  
**Blocked:** none

---

### WI-M3 — XT-IDE controller, disk image, VIDEO ROM boot (pe-afx)

**Req traceability:** REQ-M3, TS-5, TS-7, BR-5  
**Plan gaps fixed:** G-2 (`disk_image` config key fully implemented)  
**Expected files:**

- `emulator/src/disk.rs` — `DiskImage`: load raw flat binary (`disk_image` config key);
  `read_sector(lba: u32) → [u8; 512]`; `write_sector(lba, data)` (stub for M3)
- `emulator/src/xt_ide.rs` — `XtIde` at `$E300–$E30E`: BSY/DRQ/ERR bits;
  READ SECTORS `$20` state machine (BSY set → DRQ set → 512-byte transfer);
  SET FEATURES `$EF` (BSY clears; DRQ/ERR absent);
  IDENTIFY `$EC` (minimal 512-byte response);
  probe write tolerance for `$E300–$E330`; trace logging of every register access
- `emulator/src/bus.rs` — XT-IDE decode at `$E300–$E30E`;
  absent-device stubs: CH375 `$E260–$E261`, ESP `$E100–$E102`,
  Multi-I/O `$E3FE–$E3FF` (all return `open_bus` / discard writes)
- `emulator/tests/m3_xt_ide_gate.rs` — VIDEO bank reset vector = `$F000`;
  probe writes no crash; 60 sector transfers complete; PC = `$0800`

**Gate observable:** 60 sector reads succeed without error; PC = `$0800` logged.

**Verification expectations:**
1. `rom_bank = "video"`: reset vector returns `$F000`.
2. Probe writes `$FF`/`$00` to `$E300–$E330`: no crash; LBA 0 bytes unchanged.
3. SET FEATURES `$EF`: BSY clears; DRQ and ERR absent.
4. Instrument sector-transfer counter; assert reaches 60.
5. CPU PC equals `$0800` after 60th transfer.

**Risks:** R-1 (HEX bank layout) — `rom_bank` config; R-3 (XT-IDE register spacing) — trace logging.

**Dependencies:** pe-fp3  
**Skipped:** WRITE SECTORS (M5); IDENTIFY full block (M5)  
**Blocked:** none

---

### WI-M4 — DOS/65 cold boot, absent-device stubs (pe-cs9)

**Req traceability:** REQ-M4, TS-8, BR-7  
**Expected files:**

- `emulator/src/peripherals.rs` — safe no-op stubs:
  CH375 `$E260–$E261` (reads → `open_bus`; writes discarded);
  Dual ESP `$E100–$E102` (same);
  Multi-I/O `$E3FE–$E3FF` (keyboard self-test: `$AA` cmd → `$55` response;
  all other reads → `open_bus`)
- `emulator/src/bus.rs` — wire `peripherals.rs` stubs; task-switching during
  loader correctly accesses task-1 physical `$10000–$11FFF`;
  far-call stub at `$FFF0` executes from ROM (no code synthesis needed — A-6)
- `emulator/tests/m4_dos_boot_gate.rs` — physical `$B800–$D870` non-zero after
  task-0 copy; physical `$10000–$11FFF` non-zero after task-1 copy;
  stdout contains `DOS/65` then `A>`; inject `A:\r`; echo received; no hang

**Pre-implementation note (R-4):** Disassemble ROM around `$FFF0` before coding
to confirm far-call stub encoding matches BIOS source (`$FFF0` entry point).

**Gate observable:** `DOS/65` and `A>` on serial console.

**Verification expectations:**
1. Physical `$B800–$D870` (task-0 map) is non-zero after loader task-0 copy.
2. Physical `$10000–$11FFF` is non-zero after loader task-1 copy.
3. stdout contains substring `DOS/65` followed by `A>`.
4. Inject `A:\r`; assert echo; no timeout or fault.
5. SIM device-init loop: no absent-device failure halts execution.

**Risk:** R-4 (far-call stub encoding) — disassemble ROM pre-implementation.

**Dependencies:** pe-afx  
**Skipped:** none  
**Blocked:** none

---

### WI-M5 — DOS/65 disk read/write, directory listing (pe-czj)

**Req traceability:** REQ-M5, TS-5  
**Expected files:**

- `emulator/src/xt_ide.rs` — WRITE SECTORS `$30`: accept 512-byte write, commit to DiskImage;
  IDENTIFY `$EC`: full 512-byte response (fixed ASCII model string);
  bad-sector injection path: per-LBA configurable error return;
  B-drive LBA isolation (`$4100–$81FF` disjoint from A `$0000–$40FF`)
- `emulator/src/disk.rs` — `write_sector()` fully implemented;
  flush written sectors to raw image file
- `emulator/tests/m5_disk_io_gate.rs` — from `A>`: inject `DIR A:\r`;
  assert CP/M directory entry in stdout; load `.COM`; assert output, no hang;
  write small file, read back, assert byte identity; drive E → failure, no crash;
  bad-sector LBA → `BAD SECTOR` in stdout; Return → `A>` returns;
  B-drive write → A LBA 0 unchanged

**Gate observable:** `DIR A:` lists CP/M directory entries.

**Verification expectations (REQ-M5):**
1. `DIR A:` lists directory entries from CP/M filesystem.
2. `.COM` file loads and runs without hang.
3. Write then read-back of small file: byte-identical.
4. Drive E access: failure returned; no crash; no hang.
5. Bad-sector injection: `BAD SECTOR` in stdout; Return → `A>`; other key → warm boot.
6. B-drive write does not corrupt A-range sectors.

**Assumptions:** A-3 (512-byte sector), A-4 (CP/M dir at A partition `$0000–$40FF`).

**Dependencies:** pe-cs9  
**Skipped:** drive B warm-boot path (only Return key path required for acceptance)  
**Blocked:** none

---

### WI-M6 — RTC model, config-file boot, configuration hardening (pe-736)

**Req traceability:** REQ-M6, TS-6, TS-9, BR-6, BR-7, OQ-R0.5  
**Expected files:**

- `emulator/src/rtc.rs` — `Rtc` struct: 16 nibble-only registers `$EF90–$EF9F`
  (low 4 bits of each byte only); control regs at offsets `$0D–$0F`;
  three clock policies (`host`, `fixed`, `epoch`); STOP/RESET bit:
  write sequence `$02/$00/$00/$01/$05/$04` freezes then restarts counter
- `emulator/src/bus.rs` — wire `Rtc` at `$EF90–$EF9F`;
  confirm open-bus returns for `$EFA0–$EFCF`, `$EFF0–$EFFF`,
  unassigned MMU offsets, and physical holes above page `$7F`
- `emulator/src/config.rs` — full TOML config-file loading via `--config` CLI arg;
  `open_bus` documented with `$EA` as first-choice comment (R-6 mitigation)
- `emulator/config/default.toml` — `acia_cts_default = true` default (R-5 mitigation)
- `emulator/tests/m6_rtc_config_gate.rs` — `rtc_policy=host`: DOS time/date
  returns year 2020–2040; RTC write sequence no fault; `rtc_policy=fixed` with
  known epoch: date matches; CH375 `C:` → failure, `A>` returns; Multi-I/O
  `$AA`→`$55`; read `$EFA0` → `open_bus`; start with custom config file
  (`rom_bank=video`, `open_bus=0xEA`, `rtc_policy=host`): boot succeeds

**Gate observable:** RTC read returns plausible date; emulator starts from config file with non-default settings.

**Verification expectations (REQ-M6):**
1. DOS time/date command: plausible year (2020–2040) matching configured RTC policy.
2. Firmware write sequence `$02/$00/$00/$01/$05/$04`: no fault; clock advances.
3. `rtc_policy = "fixed"` with known epoch: returned date matches epoch.
4. CH375 (`C:` access): failure returned; no crash; `A>` returns.
5. Multi-I/O `$AA` command: `$55` response; init does not hang.
6. `read($EFA0)` equals `open_bus` config value.
7. `--config custom.toml` with non-default settings: boot succeeds.

**Risks:** R-5 (CTS deadlock) — `acia_cts_default = true` in default.toml;
R-6 (open-bus startup branch) — `$EA` as documented first choice.

**Dependencies:** pe-czj  
**Skipped:** video card pages `$F8–$F9`, cycle-accurate timing, floppy, ESP networking  
**Blocked:** none

---

## Coverage Matrix

| ID | Status |
| --- | --- |
| REQ-M1 | covered |
| REQ-M2 | covered |
| REQ-M3 | covered |
| REQ-M4 | covered |
| REQ-M5 | covered |
| REQ-M6 | covered |
