// Gate test for WI-M2: MMU — 64 tasks, BIOS INITPAGES, task switching.
//
// Acceptance (REQ-M2):
//   1. INITPAGES completes: $EFE4 bit 7 set (enabled), bits 5:0 = $00 (task 0).
//   2. Task-0 edit window round-trip: write 16 known bytes to $EFD0-$EFDF, read back identical.
//   3. Task-1 logical $C000 → physical $10000; $D000 → $11000 (not task-0 identity).
//   4. Banner ('_') visible in ACIA output after boot (MMU enabled, task 0 active).
//   5. Task mask: write $FF to $EFE0 → $EFE4 bits 5:0 = $3F.

use emulator::config::Config;
use emulator::emulator::Machine;

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
