// Gate test for WI-M2: MMU — 64 tasks, BIOS INITPAGES, task switching.
//
// Acceptance (REQ-M2):
//   1. INITPAGES completes: $EFE4 bit 7 set (enabled), bits 5:0 = $00 (task 0).
//   2. Task-0 edit window round-trip: write 16 known bytes to $EFD0-$EFDF, read back identical.
//   3. Task-1 logical $C000 → physical $10000; $D000 → $11000 (not task-0 identity).
//   4. Banner ('_') visible in ACIA output after boot (MMU enabled, task 0 active).
//   5. Task mask: write $FF to $EFE0 → $EFE4 bits 5:0 = $3F.

use emulator::bus::Bus;
use emulator::config::{Config, MmuPowerOnFill};
use emulator::emulator::Machine;
use emulator::rom::Rom;

/// A bus with a blank ($FF-filled) ROM and no disk, for driving MMU registers
/// directly without booting real firmware.
fn blank_bus(cfg: Config) -> Bus {
    let rom = Rom::blank(cfg.rom_bank.clone());
    Bus::new(&cfg, rom, None)
}

fn rom_hex_path() -> String {
    std::env::var("PC6502_ROM_HEX").unwrap_or_else(|_| {
        let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // Walk up from the crate root looking for PC6502_firmware_source/rom.hex.
        // Depth to the repo root varies: 1 level in the main checkout
        // (PC6502Emulator/emulator), 3 levels in a worktree
        // (PC6502Emulator/worktrees/<name>/emulator).
        manifest
            .ancestors()
            .map(|dir| dir.join("PC6502_firmware_source/rom.hex"))
            .find(|candidate| candidate.exists())
            .unwrap_or_else(|| manifest.join("PC6502_firmware_source/rom.hex"))
            .to_str()
            .unwrap()
            .to_string()
    })
}

#[test]
fn m2_initpages_and_map_roundtrip() {
    let rom_path = rom_hex_path();
    assert!(
        std::path::Path::new(&rom_path).exists(),
        "rom.hex not found at {}; set PC6502_ROM_HEX to override",
        rom_path
    );

    let mut cfg = Config::default();
    cfg.rom_hex = Some(rom_path);

    let mut machine = Machine::new(cfg);
    {
        let bus = &mut machine.bus;
        machine.cpu.reset(|addr| bus.read(addr));
    }

    // Run until the register dump (';') appears — INITPAGES has completed by then.
    const BOOT_CYCLES: u64 = 5_000_000;
    let mut total: u64 = 0;
    while total < BOOT_CYCLES {
        total += machine.step_one() as u64;
        if machine.bus.acia().output().contains(&b';') {
            break;
        }
    }

    // REQ-M2-5: Banner ('_') must be visible in ACIA output.
    assert!(
        machine.bus.acia().output().contains(&b'_'),
        "Banner '_' not seen after {}M cycles; output: {:?}",
        BOOT_CYCLES / 1_000_000,
        String::from_utf8_lossy(machine.bus.acia().output())
    );

    // REQ-M2-1: $EFE4 bit 7 = enabled; bits 5:0 = $00 (active task 0).
    let efe4 = machine.bus.read(0xEFE4);
    assert_eq!(
        efe4 & 0x80,
        0x80,
        "MMU not enabled after INITPAGES; $EFE4 = {:#04x}",
        efe4
    );
    assert_eq!(
        efe4 & 0x3F,
        0x00,
        "Active task not 0 after INITPAGES; $EFE4 = {:#04x}",
        efe4
    );

    // REQ-M2-3: Task-1 logical $C000 → physical page $10 ($10000); $D000 → $11 ($11000).
    // Task-0 keeps identity mapping for those pages.
    let t1_c = machine.bus.mmu().translate(1, 0xC);
    assert_eq!(
        t1_c, 0x10,
        "Task-1 page $C should map to physical page $10 ($10000); got {:#04x}",
        t1_c
    );
    let t1_d = machine.bus.mmu().translate(1, 0xD);
    assert_eq!(
        t1_d, 0x11,
        "Task-1 page $D should map to physical page $11 ($11000); got {:#04x}",
        t1_d
    );
    let t0_c = machine.bus.mmu().translate(0, 0xC);
    assert_eq!(
        t0_c, 0x0C,
        "Task-0 page $C should stay identity $0C; got {:#04x}",
        t0_c
    );

    // REQ-M2-2: Task-0 edit window round-trip — write 16 known bytes, read back identical.
    machine.bus.write(0xEFE1, 0x00); // set setup task = 0
    let pattern: [u8; 16] = [
        0xA5, 0x5A, 0xB6, 0x69, 0xC3, 0x3C, 0xD0, 0x0D,
        0xE1, 0x1E, 0xF2, 0x2F, 0x03, 0x30, 0x14, 0x41,
    ];
    for (i, &b) in pattern.iter().enumerate() {
        machine.bus.write(0xEFD0 + i as u16, b);
    }
    for (i, &expected) in pattern.iter().enumerate() {
        let got = machine.bus.read(0xEFD0 + i as u16);
        assert_eq!(
            got, expected,
            "Edit window byte {i}: expected {expected:#04x}, got {got:#04x}"
        );
    }

    // REQ-M2-6 / BR-3: Write $FF to $EFE0 → $EFE4 bits 5:0 = $3F (task mask).
    machine.bus.write(0xEFE0, 0xFF);
    let efe4_after = machine.bus.read(0xEFE4);
    assert_eq!(
        efe4_after & 0x3F,
        0x3F,
        "Task mask should clamp $FF to $3F; $EFE4 = {:#04x}",
        efe4_after
    );
}

// The tests below close gaps flagged in specifications/system-reference.md
// (R0.7, R1.5, R1.6, R1.7) against this bead's own file list: `io_rom_always`
// I/O/ROM decode precedence and the task-0 edit-alias compatibility option in
// mmu.rs, plus the `$EFE6`/`$EFE7` control-register offsets left unimplemented
// in the original M2 pass. They drive the bus directly (no ROM boot needed).

// OQ-R0.7: with io_rom_always=true, I/O/ROM decode is derived from the
// *physical* page after MMU translation, so remapping a low logical page onto
// physical $0E/$0F still reaches I/O/ROM instead of aliasing into RAM.
#[test]
fn io_rom_always_relocates_io_and_rom() {
    let mut cfg = Config::default();
    cfg.io_rom_always = true;
    cfg.mmu_power_on_fill = MmuPowerOnFill::Zero;
    let mut bus = blank_bus(cfg);

    // Task 0: logical $E/$F identity (so direct $EFEx / $F000 access still
    // works once io_rom_always routes every access through translation), plus
    // logical $3 -> physical $0E (I/O) and $4 -> physical $0F (ROM).
    bus.write(0xEFE1, 0x00); // setup task = 0
    bus.write(0xEFD0 + 0x3, 0x0E);
    bus.write(0xEFD0 + 0x4, 0x0F);
    bus.write(0xEFD0 + 0xE, 0x0E);
    bus.write(0xEFD0 + 0xF, 0x0F);
    bus.write(0xEFE0, 0x00); // active task = 0
    bus.write(0xEFE2, 0x01); // enable MMU

    // Logical $3FE4 -> physical $0EFE4 -> I/O page, offset $FE4 -> synthetic
    // $EFE4 (MMU status). Must match reading $EFE4 directly.
    let direct_status = bus.read(0xEFE4);
    let relocated_status = bus.read(0x3FE4);
    assert_eq!(
        relocated_status, direct_status,
        "logical $3FE4 (mapped to physical $0E) should reach the same MMU \
         status register as $EFE4; got {relocated_status:#04x} vs {direct_status:#04x}"
    );

    // Logical $4000 -> physical $0F000 -> ROM page, offset 0 -> first byte of
    // the active ROM bank ($FF in a blank test ROM).
    let relocated_rom = bus.read(0x4000);
    assert_eq!(
        relocated_rom, 0xFF,
        "logical $4000 (mapped to physical $0F) should read ROM; got {relocated_rom:#04x}"
    );

    // ROM write protection still applies through the relocated path.
    bus.write(0x4000, 0x00);
    assert_eq!(
        bus.read(0x4000),
        0xFF,
        "write through relocated ROM page must not alter ROM contents"
    );
}

// OQ-R0.7 regression: the default (false) must keep the old, hardware-confirmed
// behavior — only the identity case ($E/$F logical pages) decodes as I/O/ROM.
// A low page remapped onto physical $0E must read as plain RAM, not I/O.
#[test]
fn io_rom_always_default_false_does_not_relocate() {
    let mut cfg = Config::default();
    assert!(!cfg.io_rom_always, "default must be false");
    cfg.mmu_power_on_fill = MmuPowerOnFill::Zero;
    let mut bus = blank_bus(cfg);

    bus.write(0xEFE1, 0x00);
    bus.write(0xEFD0 + 0x3, 0x0E); // logical $3 -> physical $0E (I/O page)
    bus.write(0xEFE0, 0x00);
    bus.write(0xEFE2, 0x01); // enable MMU

    // Fresh RAM is zero-initialized; the MMU status register would instead
    // read 0x80 (enabled, task 0), so the two outcomes are unambiguous.
    let val = bus.read(0x3000);
    assert_eq!(
        val, 0x00,
        "with io_rom_always=false, a remapped low page must read plain RAM, \
         not I/O; got {val:#04x}"
    );
}

// OQ-R1.7: the task0_alias_defect compatibility mode duplicates every
// edit-window write into task 0, regardless of the selected setup task.
#[test]
fn task0_alias_defect_duplicates_edit_writes_when_enabled() {
    let mut cfg = Config::default();
    cfg.mmu_task0_alias_defect = true;
    cfg.mmu_power_on_fill = MmuPowerOnFill::Zero;
    let mut bus = blank_bus(cfg);

    bus.write(0xEFE1, 0x05); // setup task = 5 (task 0 never selected)
    for i in 0..16u16 {
        bus.write(0xEFD0 + i, 0x40 + i as u8);
    }

    for i in 0..16u8 {
        assert_eq!(bus.mmu().translate(5, i), 0x40 + i, "task 5 entry {i}");
        assert_eq!(
            bus.mmu().translate(0, i),
            0x40 + i,
            "task 0 alias entry {i} should mirror the task-5 edit-window write"
        );
    }
}

// OQ-R1.7 regression: default off leaves task 0 untouched by edits to another
// task's map, matching the "clean architectural model" the spec requires.
#[test]
fn task0_alias_defect_off_by_default_leaves_task0_untouched() {
    let mut cfg = Config::default();
    assert!(!cfg.mmu_task0_alias_defect, "default must be false");
    cfg.mmu_power_on_fill = MmuPowerOnFill::Zero;
    let mut bus = blank_bus(cfg);

    bus.write(0xEFE1, 0x05); // setup task = 5
    for i in 0..16u16 {
        bus.write(0xEFD0 + i, 0x40 + i as u8);
    }

    for i in 0..16u8 {
        assert_eq!(
            bus.mmu().translate(0, i),
            0x00,
            "task 0 entry {i} must stay at its power-on fill; alias mode is off"
        );
    }
}

// OQ-R1.5: reading $EFE6 asserts an observable ISA TC signal event. The
// returned byte value is undefined on hardware, so only the side effect
// (not the read data) is checked.
#[test]
fn efe6_read_asserts_observable_tc_signal() {
    let cfg = Config::default();
    let mut bus = blank_bus(cfg);
    assert_eq!(bus.mmu().tc_signal_count(), 0);
    let _ = bus.read(0xEFE6);
    let _ = bus.read(0xEFE6);
    let _ = bus.read(0xEFE6);
    assert_eq!(
        bus.mmu().tc_signal_count(),
        3,
        "each $EFE6 read should assert one observable TC signal event"
    );
}

// OQ-R1.6: $EFE7 bits 3:0 report the current I/O physical page ($0E in the
// firmware-compatible default); upper bits are documented as not meaningful.
#[test]
fn efe7_reports_io_physical_page_low_nibble() {
    let cfg = Config::default();
    let mut bus = blank_bus(cfg);
    let val = bus.read(0xEFE7);
    assert_eq!(
        val & 0x0F,
        0x0E,
        "EFE7 low nibble should report the I/O physical page ($0E); got {val:#04x}"
    );
}
