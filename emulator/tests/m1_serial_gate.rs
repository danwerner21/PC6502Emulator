// Gate test for WI-M1: CPU core, flat bus, ACIA serial output.
//
// Acceptance:
//   1. Supermon '>' appears on stdout within 10M cycles after reset (Base ROM bank).
//      Supermon v1.2 does not print a spontaneous prompt; it blocks on ACIA RX.
//      We inject '> F000\r' after the register dump appears so the CPU echoes '>'
//      and executes the examine command — this is the ACIA RX/TX round-trip proof.
//   2. 'G F000\r' re-runs the BIOS (jumps to $F000), banner re-appears.

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
fn m1_supermon_prompt() {
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

    // Phase 1: run until the register dump ('; ') appears in ACIA output.
    // Supermon v1.2 prints the banner + register dump and then blocks on ACIA RX;
    // it does not emit a spontaneous prompt.  Allow up to 5M cycles.
    const BOOT_CYCLES: u64 = 5_000_000;
    let mut total: u64 = 0;
    while total < BOOT_CYCLES {
        total += machine.step_one() as u64;
        if machine.bus.acia().output().contains(&b';') {
            break;
        }
    }

    assert!(
        machine.bus.acia().output().contains(&b';'),
        "Register dump (';') not seen within {}M cycles; output so far: {:?}",
        BOOT_CYCLES / 1_000_000,
        String::from_utf8_lossy(machine.bus.acia().output())
    );

    // Phase 2: inject '> F000\r' — Supermon echoes '>' (ACIA RX/TX round-trip)
    // and executes the examine command.  Look for '>' within another 5M cycles.
    machine.bus.acia_mut().inject_rx_bytes(b"> F000\r");

    const MAX_CYCLES: u64 = 5_000_000;
    total = 0;
    let mut first_prompt_pos: Option<usize> = None;
    while total < MAX_CYCLES {
        total += machine.step_one() as u64;
        let out = machine.bus.acia().output();
        if first_prompt_pos.is_none() && out.contains(&b'>') {
            first_prompt_pos = Some(out.len());
            break;
        }
    }

    assert!(
        first_prompt_pos.is_some(),
        "No '>' seen after injecting '> F000\\r' within {}M cycles; output: {:?}",
        MAX_CYCLES / 1_000_000,
        String::from_utf8_lossy(machine.bus.acia().output())
    );

    // Phase 3: inject 'G F000\r' — Supermon Go command jumps to $F000 (re-runs BIOS).
    // The banner (containing '_') must re-appear.
    machine.bus.acia_mut().inject_rx_bytes(b"G F000\r");

    let output_before = machine.bus.acia().output().len();
    let mut saw_banner = false;
    total = 0;
    while total < MAX_CYCLES {
        total += machine.step_one() as u64;
        let out = machine.bus.acia().output();
        // The ASCII-art banner contains many '_' characters.
        if out.len() > output_before && out[output_before..].contains(&b'_') {
            saw_banner = true;
            break;
        }
    }

    assert!(
        saw_banner,
        "'G F000' did not re-emit the banner within {}M cycles; new output: {:?}",
        MAX_CYCLES / 1_000_000,
        String::from_utf8_lossy(&machine.bus.acia().output()[output_before..])
    );
}
