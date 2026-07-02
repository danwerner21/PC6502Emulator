# Artifact Verification and Integration Requirements

**Bead**: pe-by4 (corrects mc-0sj / investigator-2 version)  
**Date**: 2026-07-02  
**Produced by**: investigator-adhoc-e8fcdb1f10  

---

## 1. Artifact Status

### 1.1 `emulator/disk_image/disk.img`

| Property | Value |
|---|---|
| Size | 8,519,680 bytes |
| Format | Raw flat binary (confirmed; `file` reports "data") |
| Sector size | 512 bytes |
| Sector count | **16,640** (= 0x4100; exactly one DOS/65 slice) |
| Created | 2026-06-22 |

**LBA 0 content:**  
LBA 0 contains 6502 machine code. The first 8 bytes (`$A9 $00 $8D $E0 $EF $A9 $01 $8D`) match the first record of `pcdos65.s19` at $0800 (`LDA #$00 / STA $EFE0`). This is the boot loader, not a CP/M directory.

**Disk layout:**

| Region | LBA range | Content |
|---|---|---|
| Boot load area | 0–59 | 60 sectors of DOS/65 OS payload; ROM copies to $0800–$7FFF |
| Reserved system tracks (remainder) | 60–255 | $E5-filled; unused portion of the 16-track reserved area |
| CP/M directory + data | 256–16,639 (0x0100+) | Active directory and file allocation area |

**⚠ Correction to prior report (mc-0sj):** The mc-0sj version identified LBA 60 as the CP/M directory start and concluded the disk was empty. This is wrong. Per the DOS/65 spec (16 reserved tracks × 64 logical sectors × 128 bytes / 512 = 256 physical sectors), the directory starts at LBA **0x0100 (256)**. LBAs 60–255 are the tail of the reserved system area, not the directory.

**CP/M directory at LBA 0x0100 — 29 active files:**

| Filename | Filename | Filename |
|---|---|---|
| ALLOC.COM | EDIT.COM | RUN.COM |
| ASM.COM | MICROCHE | S19.COM |
| ASSIGN.COM | MKCOM.COM | SEDIT.COM |
| BCOMPILE.COM | NVSET.COM | SPSC.COM |
| COMPARE.COM | PASCAL.COM | TREK.BAS |
| COPY.COM | PRPDRV.COM | ULTIMA.COM |
| DBASIC.COM | PRUN.COM | WYRMHOLD.COM |
| DBASICMA | — | XMR.COM |
| DBASICMP.COM | — | XMS.COM |
| DEBUG.COM | — | ZIP.COM |
| — | — | ZIPTEST.Z3 |
| — | — | ZORK1.Z3 |

Directory holds 29 active entries; 481 deleted/unused ($E5) entries; 512 total slots.

**Boot area:** LBAs 0–59 (60 sectors). The loader code at LBA 0 matches the $0800 S-record segment byte-for-byte.

---

### 1.2 `emulator/disk_image/pcdos65.s19`

| Property | Value |
|---|---|
| Size | 32,716 bytes |
| Format | Valid Motorola S-record (all records type S1; terminator S9) |
| Record count | 437 data records + 1 S9 terminator |
| Address range | $0800 – $55B0 |
| Execution address (S9) | **$0800** |

**Segment breakdown:**

| Segment | S-record range | Bytes | Notes |
|---|---|---|---|
| Loader | $0800 – $085E | 95 | Boot entry point; MMU init + copy loop |
| Loader gap | $085F – $0FFF | — | Not in S-record; zero-filled in sectors |
| DOS/65 main | $1000 – $3070 | 8,305 | Copied by loader to task-0 $B800–$D870 |
| DOS/65 gap | $3071 – $3FFF | — | Not in S-record; zero-filled in sectors |
| Driver code | $4000 – $55B0 | 5,553 | Copied by loader to task-1 $C000–$DFFF → physical $10000–$11FFF |

**Alignment with REQ-M4-1/2:**

- **REQ-M4-1 (task-0 copy to $B800–$D870):** ✓ S-record contains code at $1000–$3070. The loader copies $1000–$37FF → $B800–$D870; bytes at $3071–$3FFF will be zero, filling the tail of the copy window. Physical $B800–$D870 will be non-zero after the copy (code present from $1000).
- **REQ-M4-2 (task-1 copy to $10000–$11FFF):** ✓ S-record contains driver code at $4000–$55B0. The loader copies $4000–$5FFF → task-1 $C000–$DFFF → physical $10000–$11FFF. Bytes at $55B1–$5FFF will be zero. Physical $10000–$11FFF will be non-zero (driver present from $4000).

---

### 1.3 `emulator/disk_image/rom.hex`

| Property | Value |
|---|---|
| Size | 19,468 bytes |
| Format | Intel HEX (ASCII text) |
| Also present at | `PC6502_firmware_source/rom.hex` |
| Files identical | **Yes** (SHA-256 match) |

`rom.hex` is the prebuilt Base+VIDEO ROM required by REQ-M3-5. It already exists in `PC6502_firmware_source/` where the M1 and M2 gate tests resolve it via `CARGO_MANIFEST_DIR` parent traversal. The copy in `emulator/disk_image/` is a duplicate placed for convenience.

---

## 2. Deferred Sub-Criteria Readiness Checklist

### REQ-M3-5 — Serial banner (VIDEO ROM UART init)

> "Serial banner appears (VIDEO bank also initializes UART)."

| Item | Status |
|---|---|
| `rom.hex` available | ✓ PRESENT (`PC6502_firmware_source/rom.hex` and `emulator/disk_image/rom.hex` — identical) |
| `disk.img` available | ✓ PRESENT (60 sectors of boot code; loader at $0800) |
| M3 gate test uses real rom.hex | ✗ NOT YET (uses `build_video_rom()` synthetic ROM) |
| M3 gate test uses real disk.img | ✗ NOT YET (uses `DiskImage::blank(100)`) |

**Assessment: READY TO IMPLEMENT.** An additional test section must load real `rom.hex` with `RomBank::Video` and real `disk.img`, run until a banner character appears in ACIA output, and assert non-empty output before PC reaches $0800.

---

### REQ-M4-3 — Far-call dispatcher

> "Far call $FFF0 → dispatcher $C000 in task 1 → returns to task-0 caller with A preserved."

| Item | Status |
|---|---|
| `disk.img` with loader code | ✓ PRESENT (loader at LBA 0–59, entry $0800) |
| `pcdos65.s19` driver at $4000–$55B0 | ✓ PRESENT |
| M4 gate test uses real files | ✗ NOT YET (uses `build_dos65_boot_rom()` synthetic ROM) |
| Far-call stub at $FFF0 in ROM | Needs verification in real rom.hex |

**Assessment: READY TO IMPLEMENT.** The far-call stub at $FFF0 is part of the Base ROM (rom.hex). The real boot path (VIDEO ROM → 60-sector load → $0800 loader → task-0/task-1 copies) must be exercised with real artifacts to test REQ-M4-3. The existing M4 test verifies physical memory layout via a synthetic shortcut but does not exercise the actual far-call ROM stub.

---

### REQ-M4-4 — SIM cold-init

> "SIM cold init loop completes: no device-init failure halts execution."

| Item | Status |
|---|---|
| `disk.img` required | ✓ PRESENT |
| DOS/65 driver code (handles device init) | ✓ Present at $4000–$55B0 in pcdos65.s19 / LBAs 0–59 |
| M4 gate test uses real files | ✗ NOT YET |

**Assessment: READY TO IMPLEMENT.** The SIM cold-init loop is real DOS/65 code that runs after the real boot path. Requires loading disk.img into the emulator and running the full boot sequence. The existing M4 synthetic ROM does not implement the real SIM cold-init path.

---

### REQ-M5-1 — DIR A: listing

> "`DIR A:` on the DOS prompt lists directory entries from the CP/M filesystem."

| Item | Status |
|---|---|
| `disk.img` available | ✓ PRESENT |
| CP/M directory location | LBA 0x0100 (256) — correct per DOS/65 spec |
| Actual files in CP/M directory | ✓ **29 FILES PRESENT** (see §1.1) |
| M5 gate test uses real disk.img | ✗ NOT YET (uses `DiskImage::blank(20000)`) |

**Assessment: READY TO IMPLEMENT.** Disk contains 29 real CP/M files. `DIR A:` will produce a populated listing. No disk-population step required.

**Note:** The prior mc-0sj report concluded "PARTIALLY READY — no files on disk." That was wrong; it checked LBA 60 (reserved system area) instead of LBA 256 (actual directory).

---

### REQ-M5-2 — .COM load

> "A valid `.COM` program loads and runs without hanging."

| Item | Status |
|---|---|
| `disk.img` available | ✓ PRESENT |
| .COM files on disk | ✓ **24 .COM FILES PRESENT** (ALLOC, ASM, ASSIGN, BCOMPILE, COMPARE, COPY, DBASIC, DBASICMP, DEBUG, EDIT, MKCOM, NVSET, PASCAL, PRPDRV, PRUN, RUN, S19, SEDIT, SPSC, ULTIMA, WYRMHOLD, XMR, XMS, ZIP) |
| M5 gate test uses real disk.img | ✗ NOT YET |

**Assessment: READY TO IMPLEMENT.** Multiple .COM files are present. `S19.COM` (small, single extent) is a suitable minimal test candidate.

**Note:** The prior mc-0sj report concluded "BLOCKED — no .COM files." This was wrong for the same directory-location reason.

---

### REQ-M5-4 — Drive-E failure

> "Accessing drive E returns failure; no crash or hang; DOS prompt returns."

| Item | Status |
|---|---|
| `disk.img` available | ✓ PRESENT |
| Full DOS/65 boot path required | ✓ Achievable with disk.img + rom.hex |
| M5 gate test uses real disk.img | ✗ NOT YET |

**Assessment: READY TO IMPLEMENT.** Drive-E failure behavior is determined by DOS/65 runtime code, not by disk content. Once the full boot path is operational with real artifacts, injecting a `E:` command via ACIA and asserting no crash is straightforward.

---

## 3. Code Changes Required Per Criterion

### REQ-M3-5

**File:** `emulator/tests/m3_xt_ide_gate.rs`

**Changes:**
1. Add a helper `disk_image_path()` using `PC6502_DISK_IMG` env var with fallback to `format!("{}/disk_image/disk.img", env!("CARGO_MANIFEST_DIR"))`.
2. Add a helper `rom_hex_path()` using `PC6502_ROM_HEX` env var with fallback to the existing three-parent traversal (matching m1/m2 pattern).
3. Add a new test section (or separate `#[test]` fn) that:
   - Loads real rom.hex with `RomBank::Video` selected.
   - Loads real disk.img via `DiskImage::load(&disk_image_path())`.
   - Resets the CPU and runs until either (a) a banner character (e.g., `_` or the PC6502 ASCII art) appears in ACIA output, or (b) PC reaches $0800.
   - Asserts that ACIA output is non-empty before PC=$0800 (confirming the VIDEO ROM initialized the UART and printed a banner).
4. Existing `DiskImage::blank(100)` calls in the current test may remain unchanged (they cover the XT-IDE unit-test section independently).

---

### REQ-M4-3 and REQ-M4-4

**File:** `emulator/tests/m4_dos_boot_gate.rs`

**Changes:**
1. Add `disk_image_path()` helper (same pattern as above).
2. Add `rom_hex_path()` helper (same pattern as m2).
3. Add a new `#[test]` fn `m4_real_boot_far_call_and_sim_init()` that:
   - Loads real rom.hex (Base bank) and real disk.img.
   - Resets and runs until `A>` appears in ACIA output (or timeout at 5M+ cycles).
   - **REQ-M4-3**: After `A>` appears, confirm that the far-call path was exercised. This may require a bus trace or a test that calls the far-call stub directly. Alternatively, assert that physical $10000 and $11FFF contain non-zero values (loader placed them there) and that DOS/65 is running (implying far-call returned cleanly).
   - **REQ-M4-4**: Assert that the `A>` prompt appeared without a crash or hang, which confirms SIM cold-init completed.
4. The existing synthetic ROM test (`m4_dos65_cold_boot_prompt`) must be preserved — it tests REQ-M4-1/2/5/6 and runs without external files.

---

### REQ-M5-1, REQ-M5-2, REQ-M5-4

**File:** `emulator/tests/m5_disk_io_gate.rs`

**Changes (Section 1 — XT-IDE unit tests):**
- `DiskImage::blank(20000)` in the standalone Bus tests (1a–1e) should remain as-is; these tests validate the XT-IDE model independently and do not need real disk content.

**Changes (Section 2 — CPU-driven write):**
- `DiskImage::blank(20000)` in the Machine test may remain as-is; it writes to LBA 1 and reads back, which only requires a writable blank image.

**New Section 3 — Full DOS/65 integration (REQ-M5-1, M5-2, M5-4):**
1. Add `disk_image_path()` and `rom_hex_path()` helpers.
2. Add `#[test]` fn `m5_dir_listing_and_drive_e_failure()`:
   - Load real `rom.hex` (Base bank) and real `disk.img`.
   - Boot to `A>` prompt.
   - **REQ-M5-1**: Inject `DIR A:\r` via ACIA; run until output changes; assert output contains directory entries (29 files are present; expect names like `ZORK1`).
   - **REQ-M5-4**: Inject `E:\r` via ACIA; run until output changes; assert no hang and `A>` returns.
3. Add `#[test]` fn `m5_com_load()`:
   - Load real `rom.hex` and real `disk.img`.
   - Boot to `A>`, inject a small `.COM` name (e.g., `S19\r`), assert program runs without hang.

---

## 4. Disk Image Relative Path

From the `emulator/` crate root (`CARGO_MANIFEST_DIR`), the disk image is at:

```
disk_image/disk.img
```

No parent-directory traversal is needed, unlike `rom.hex` which lives outside the `emulator/` subtree.

Recommended path resolver for tests (mirrors the `PC6502_ROM_HEX` pattern in m1/m2):

```rust
fn disk_image_path() -> String {
    std::env::var("PC6502_DISK_IMG").unwrap_or_else(|_| {
        format!("{}/disk_image/disk.img", env!("CARGO_MANIFEST_DIR"))
    })
}
```

**Worktree note:** In a `worktrees/<branch>/emulator/` layout, `CARGO_MANIFEST_DIR` resolves to the worktree's emulator directory. The `disk_image/` subdirectory is present only in the main checkout. Tests should use the `PC6502_DISK_IMG` env var override when running from a worktree, or the disk_image directory should be symlinked into the worktree. The env var override provides the escape hatch without requiring the binary to be committed to every branch.

---

## 5. Summary Table

| Criterion | Artifact available | Empty/partial | Code change needed | Status |
|---|---|---|---|---|
| REQ-M3-5 (VIDEO UART banner) | rom.hex ✓, disk.img ✓ | — | New test section in m3 | **Ready to implement** |
| REQ-M4-3 (far-call dispatcher) | rom.hex ✓, disk.img ✓ | — | New integration test in m4 | **Ready to implement** |
| REQ-M4-4 (SIM cold-init) | rom.hex ✓, disk.img ✓ | — | New integration test in m4 | **Ready to implement** |
| REQ-M5-1 (DIR A: listing) | disk.img ✓ | **29 files present ✓** | New integration test in m5 | **Ready to implement** |
| REQ-M5-2 (.COM load) | disk.img ✓ | **24 .COM files present ✓** | New integration test in m5 | **Ready to implement** |
| REQ-M5-4 (drive-E failure) | disk.img ✓ | — | New integration test in m5 | **Ready to implement** |

All six deferred sub-criteria are **ready to implement** — no disk-population step needed.

---

## 6. Incidental Observations

- **All six deferred sub-criteria unblocked.** The prior mc-0sj report concluded REQ-M5-1 was "partially ready" and REQ-M5-2 was "blocked." Those assessments were based on checking LBA 60 (reserved system area, correctly $E5-filled) rather than LBA 256 (actual CP/M directory, populated with 29 files). No disk-population follow-up task is needed.
- **rom.hex duplicated.** `emulator/disk_image/rom.hex` is byte-for-byte identical to `PC6502_firmware_source/rom.hex` (SHA-256: `0bc03df6...ea42ff08f`). The `PC6502_firmware_source/` copy is canonical (referenced by M1/M2 tests). The `disk_image/` copy may be removed or replaced with a symlink.
- **Drive-B coverage.** disk.img is 16,640 sectors — exactly one DOS/65 slice (drive A, LBAs 0–$40FF). Drive B requires LBAs $4100–$81FF (16,640–33,279), beyond the image end. The emulator's `write_sector` auto-extends in memory (`self.data.resize(…)`), so the M5 B-range test passes in memory but drive-B data is not persisted in the committed disk.img.
- **pcdos65.s19 is a provisioning reference, not a test input.** The S-record and disk.img may be from different build runs. The S-record documents the intended staging layout; disk.img is the authoritative runtime artifact.
