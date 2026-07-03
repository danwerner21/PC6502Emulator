// Gate test for WI-M4: DOS/65 cold boot — task-switching, absent-device stubs, A> gate.
//
// Acceptance:
//   1. Absent-device stubs: CH375 returns 0x00 (no device), ESP returns DOS/65-compatible
//      status bytes ($E102=0x09, $E100=0x01), Multi-I/O kbd self-test ($AA→$55) works;
//      all writes to absent devices are discarded.
//   2. physical $B800-$D870 non-zero after task-0 copy.
//   3. physical $10000-$11FFF non-zero after task-1 copy via MMU task-1 map.
//   4. stdout contains "DOS/65" then "A>".
//   5. inject "A:\r"; echo appears; no hang.
//   REQ-M4-3/4. Real boot with disk.img: far-call returns, SIM init completes, A> appears.

use emulator::bus::Bus;
use emulator::config::{Config, RomBank};
use emulator::cpu::Cpu;
use emulator::disk::DiskImage;
use emulator::emulator::Machine;
use emulator::rom::Rom;

fn rom_hex_path() -> String {
    std::env::var("PC6502_ROM_HEX").unwrap_or_else(|_| {
        let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("PC6502_firmware_source/rom.hex")
            .to_str()
            .unwrap()
            .to_string()
    })
}

fn disk_image_path() -> String {
    std::env::var("PC6502_DISK_IMG").unwrap_or_else(|_| {
        format!("{}/disk_image/disk.img", env!("CARGO_MANIFEST_DIR"))
    })
}

// Synthetic base ROM that simulates the DOS/65 cold boot sequence.
//
// Boot sequence (all code at $F000):
//   Phase 1  $F000: set setup_task=0 (default); write identity map to task-0 pages 0–15
//   Phase 2  $F010: set setup_task=1 via $EFE1; write task-1 pages C→$10, D→$11 only
//              (edit window always writes to setup_task's map, NOT active_task's)
//   Phase 3  $F01F: enable MMU via $EFE2; set active_task=0 via $EFE0
//   Phase 4  $F029: task-0 copy — STA #$DE to $B800/$B801; STA #$AD to $D870
//   Phase 5  $F036: set active_task=1; STA #$C0 to $C000/$C001 (→ phys $10000/$10001)
//              STA #$EF to $DFFF (→ phys $11FFF)
//   Phase 6  $F048: active_task back to 0
//   Phase 7  $F04D: output "DOS/65\r\nA>" to ACIA
//   Phase 8  $F05B: input loop — poll RDRF, read RX, echo TX, repeat
//
// Message table at $F080: "DOS/65\r\nA>\0" (11 bytes).
// Reset vector at ROM[$FFC/$FFD] = $00/$F0 → $F000.
#[rustfmt::skip]
fn build_dos65_boot_rom() -> Rom {
    let mut base = [0xFFu8; 4096];

    let boot: &[u8] = &[
        // === Phase 1: write identity map for task-0 pages 0–15 via setup_task=0 ===
        // $F000
        0xA9, 0x00,             // LDA #$00
        0x8D, 0xE0, 0xEF,      // STA $EFE0          ; active_task = 0 (redundant but explicit)
        0xA2, 0x00,             // LDX #$00
        // $F007: loop0
        0x8A,                   // TXA
        0x9D, 0xD0, 0xEF,      // STA $EFD0,X        ; task-0[page X] = X (setup_task=0)
        0xE8,                   // INX
        0xE0, 0x10,             // CPX #$10
        0xD0, 0xF7,             // BNE loop0          ; $F010 + $F7(-9) = $F007

        // === Phase 2: point edit window at task-1; write pages C→$10, D→$11 ===
        // $F010  (edit window uses setup_task from $EFE1, not active_task from $EFE0)
        0xA9, 0x01,             // LDA #$01
        0x8D, 0xE1, 0xEF,      // STA $EFE1          ; setup_task = 1 → edit window → task-1 map
        0xA9, 0x10,             // LDA #$10           ; physical page $10 ($40 KiB)
        0x8D, 0xDC, 0xEF,      // STA $EFDC          ; task-1 page $C → phys $10
        0xA9, 0x11,             // LDA #$11           ; physical page $11 ($44 KiB)
        0x8D, 0xDD, 0xEF,      // STA $EFDD          ; task-1 page $D → phys $11

        // === Phase 3: enable MMU; set active_task=0 ===
        // $F01F
        0xA9, 0x01,             // LDA #$01
        0x8D, 0xE2, 0xEF,      // STA $EFE2          ; enable MMU
        0xA9, 0x00,             // LDA #$00
        0x8D, 0xE0, 0xEF,      // STA $EFE0          ; active_task = 0

        // === Phase 4: task-0 copy — write to physical $B800/$B801/$D870 ===
        // $F029
        0xA9, 0xDE,             // LDA #$DE
        0x8D, 0x00, 0xB8,      // STA $B800          ; phys $B800 (task-0 identity)
        0x8D, 0x01, 0xB8,      // STA $B801          ; phys $B801
        0xA9, 0xAD,             // LDA #$AD
        0x8D, 0x70, 0xD8,      // STA $D870          ; phys $D870

        // === Phase 5: task-1 copy — switch to task-1; write $C000/$C001/$DFFF ===
        // $F036
        0xA9, 0x01,             // LDA #$01
        0x8D, 0xE0, 0xEF,      // STA $EFE0          ; active_task = 1 (task-1 pages C/D map to phys $10/$11)
        0xA9, 0xC0,             // LDA #$C0
        0x8D, 0x00, 0xC0,      // STA $C000          ; page C off $000 → phys $10000
        0x8D, 0x01, 0xC0,      // STA $C001          ; phys $10001
        0xA9, 0xEF,             // LDA #$EF
        0x8D, 0xFF, 0xDF,      // STA $DFFF          ; page D off $FFF → phys $11FFF

        // === Phase 6: return to task-0 ===
        // $F048
        0xA9, 0x00,             // LDA #$00
        0x8D, 0xE0, 0xEF,      // STA $EFE0          ; active_task = 0

        // === Phase 7: output "DOS/65\r\nA>" via ACIA at $EF84 ===
        // $F04D
        0xA2, 0x00,             // LDX #$00
        // $F04F: output_loop
        0xBD, 0x80, 0xF0,      // LDA $F080,X        ; byte from message table
        0xF0, 0x07,             // BEQ $F05B          ; null terminator → rx_wait ($F054+7=$F05B)
        0x8D, 0x84, 0xEF,      // STA $EF84          ; transmit byte
        0xE8,                   // INX
        0x4C, 0x4F, 0xF0,      // JMP $F04F

        // === Phase 8: input loop — poll RDRF, echo each byte ===
        // $F05B: rx_wait
        0xAD, 0x85, 0xEF,      // LDA $EF85          ; read ACIA status
        0x29, 0x08,             // AND #$08           ; RDRF (bit 3)
        0xF0, 0xF9,             // BEQ $F05B          ; no byte: $F062+$F9(-7)=$F05B
        0xAD, 0x84, 0xEF,      // LDA $EF84          ; read RX data (clears RDRF)
        0x8D, 0x84, 0xEF,      // STA $EF84          ; echo TX
        0x4C, 0x5B, 0xF0,      // JMP $F05B          ; back to rx_wait
    ];
    base[..boot.len()].copy_from_slice(boot);

    // Message table at ROM offset $080 (CPU $F080): "DOS/65\r\nA>" + null
    let msg: &[u8] = &[0x44, 0x4F, 0x53, 0x2F, 0x36, 0x35, 0x0D, 0x0A, 0x41, 0x3E, 0x00];
    base[0x080..0x080 + msg.len()].copy_from_slice(msg);

    // Reset vector at ROM[$FFC/$FFD] = $F000 (little-endian)
    base[0xFFC] = 0x00;
    base[0xFFD] = 0xF0;

    Rom::from_banks(base, [0xFFu8; 4096], RomBank::Base)
}

#[test]
fn m4_dos65_cold_boot_prompt() {
    // === Section 1: absent-device stub unit tests on a standalone Bus ===
    let cfg = Config::default();
    let mut bus = Bus::new(&cfg, build_dos65_boot_rom(), None);

    // CH375 $E260-$E261: returns 0x00 (no USB device present); writes discarded
    assert_eq!(bus.read(0xE260), 0x00, "CH375 $E260 must return 0x00 (no device)");
    assert_eq!(bus.read(0xE261), 0x00, "CH375 $E261 must return 0x00 (no device)");
    bus.write(0xE260, 0xFF);
    bus.write(0xE261, 0xFF);
    assert_eq!(bus.read(0xE260), 0x00, "CH375 write must be discarded");

    // Dual ESP $E100-$E102: returns DOS/65-compatible status bytes; writes discarded.
    // $E102 (status) = 0x09: bit3=1 (tx-ready), bit0=1 (rx-data).
    // $E100 (data) = 0x01: non-zero/non-$FF so fn10 returns success without looping.
    assert_eq!(bus.read(0xE100), 0x01, "ESP $E100 must return 0x01");
    assert_eq!(bus.read(0xE102), 0x09, "ESP $E102 status must return 0x09");
    bus.write(0xE100, 0xFF);
    assert_eq!(bus.read(0xE100), 0x01, "ESP write must be discarded");

    // Multi-I/O $E3FE-$E3FF: keyboard self-test $AA → $55; other data-port reads
    // return open_bus ($EA). $E3FF (KBD_ST) models bit1 (busy, always clear) and
    // bit0 (data-pending); other bits mirror open_bus.
    // Real firmware writes $AA to KBD_CMD ($E3FF, offset 1; bios_multi.asm:173-174,292)
    // and reads the $55 response from KBD_DAT ($E3FE, offset 0; bios_multi.asm:176,361).
    let open_bus = cfg.open_bus.value;
    assert_eq!(
        bus.read(0xE3FF) & 0x03,
        0x00,
        "Multi-I/O $E3FF must show not-busy/no-data-pending before any command"
    );
    bus.write(0xE3FF, 0xAA); // self-test command on offset 1 (command port)
    assert_eq!(
        bus.read(0xE3FF) & 0x03,
        0x01,
        "Multi-I/O $E3FF must show not-busy/data-pending after $AA armed"
    );
    assert_eq!(bus.read(0xE3FE), 0x55, "Multi-I/O kbd self-test must return $55");
    assert_eq!(bus.read(0xE3FE), open_bus, "Multi-I/O subsequent read must return open_bus");
    assert_eq!(
        bus.read(0xE3FF) & 0x03,
        0x00,
        "Multi-I/O $E3FF must show not-busy/no-data-pending after $55 consumed"
    );

    // === Section 2: full emulation — synthetic DOS/65 boot ROM ===
    let mut machine = Machine::from_parts(
        Cpu::new(),
        Bus::new(&Config::default(), build_dos65_boot_rom(), None),
    );
    {
        let bus = &mut machine.bus;
        machine.cpu.reset(|addr| bus.read(addr));
    }
    assert_eq!(machine.cpu.pc, 0xF000, "PC after reset must be $F000");

    // Run until "A>" appears in ACIA output (output phase completes)
    const MAX_CYCLES: u64 = 500_000;
    let mut total: u64 = 0;
    loop {
        total += machine.step_one() as u64;
        if machine.bus.acia().output().windows(2).any(|w| w == b"A>") {
            break;
        }
        assert!(
            total < MAX_CYCLES,
            "timeout after {} cycles waiting for 'A>'; output: {:?}",
            total,
            String::from_utf8_lossy(machine.bus.acia().output())
        );
    }

    // REQ-M4-3: stdout must contain "DOS/65" followed by "A>"
    let output = machine.bus.acia().output().to_vec();
    let dos65_pos = output
        .windows(6)
        .position(|w| w == b"DOS/65")
        .expect("output must contain 'DOS/65'");
    let a_prompt_pos = output
        .windows(2)
        .position(|w| w == b"A>")
        .expect("output must contain 'A>'");
    assert!(
        a_prompt_pos > dos65_pos,
        "'A>' must appear after 'DOS/65' in output"
    );

    // REQ-M4-1/2: physical $B800, $B801, $D870 non-zero after task-0 copy
    assert_ne!(
        machine.bus.phys_read(0xB800), 0,
        "physical $B800 must be non-zero after task-0 copy"
    );
    assert_ne!(
        machine.bus.phys_read(0xB801), 0,
        "physical $B801 must be non-zero after task-0 copy"
    );
    assert_ne!(
        machine.bus.phys_read(0xD870), 0,
        "physical $D870 must be non-zero after task-0 copy"
    );

    // REQ-M4-4/5: physical $10000, $10001, $11FFF non-zero after task-1 copy
    assert_ne!(
        machine.bus.phys_read(0x10000), 0,
        "physical $10000 must be non-zero after task-1 copy"
    );
    assert_ne!(
        machine.bus.phys_read(0x10001), 0,
        "physical $10001 must be non-zero after task-1 copy"
    );
    assert_ne!(
        machine.bus.phys_read(0x11FFF), 0,
        "physical $11FFF must be non-zero after task-1 copy"
    );

    // REQ-M4-6: inject "A:\r"; assert echo; no hang (TS-8, BR-7)
    machine.bus.acia_mut().inject_rx_bytes(b"A:\r");
    let output_before = machine.bus.acia().output().len();
    let mut echo_seen = false;
    for _ in 0..100_000u64 {
        machine.step_one();
        let new_out = &machine.bus.acia().output()[output_before..];
        if new_out.len() >= 3 && new_out[..3] == [b'A', b':', b'\r'] {
            echo_seen = true;
            break;
        }
    }
    assert!(
        echo_seen,
        "echo of 'A:\\r' not seen after inject; new output: {:?}",
        String::from_utf8_lossy(&machine.bus.acia().output()[output_before..])
    );
}

// REQ-M4-3 + REQ-M4-4: Full DOS/65 boot with real rom.hex (Video bank) and real
// disk.img.  Verifies that the far-call dispatcher (REQ-M4-3) returned cleanly
// and that the SIM cold-init loop (REQ-M4-4) completed without hanging.
//
// Observable evidence: physical $B800 non-zero (task-0 DOS main copied),
// physical $10000 non-zero (task-1 driver copied), and "A>" appears in ACIA
// output (DOS/65 is alive and serving the command prompt).
//
// Skips cleanly if rom.hex or disk.img are not present.
#[test]
fn m4_real_boot_far_call_and_sim_init() {
    let rom_path = rom_hex_path();
    let disk_path = disk_image_path();

    if !std::path::Path::new(&rom_path).exists() || !std::path::Path::new(&disk_path).exists() {
        eprintln!(
            "m4_real_boot_far_call_and_sim_init: skipped (rom={}, disk={})",
            rom_path, disk_path
        );
        return;
    }

    let rom = Rom::load_hex(&rom_path, RomBank::Video)
        .expect("failed to load rom.hex for Video bank");
    let disk = DiskImage::load(&disk_path).expect("failed to load disk.img");

    let mut machine = Machine::from_parts(Cpu::new(), Bus::new(&Config::default(), rom, Some(disk)));
    {
        let bus = &mut machine.bus;
        machine.cpu.reset(|addr| bus.read(addr));
    }

    // Run until "A>" appears in ACIA output or timeout.
    // Full boot: VIDEO ROM → 60-sector load → $0800 loader → task copies → DOS/65 A>.
    //
    // DOS/65 routes all console output through ZP $3A (device-function base).  The
    // Video ROM sets $3A = $13 (fn 19 = ESP WiFi), so init output goes to ESP, not
    // ACIA.  After the Video ROM banner appears in ACIA (proving the UART was
    // initialised), we redirect $3A to $04 (fn 4 = ACIA TX) so that all subsequent
    // DOS/65 output — including the A> prompt — lands in the ACIA capture buffer.
    const MAX_CYCLES: u64 = 50_000_000;
    let mut total: u64 = 0;
    let mut a_prompt_seen = false;
    let mut acia_redirected = false;
    while total < MAX_CYCLES {
        total += machine.step_one() as u64;
        if !acia_redirected && !machine.bus.acia().output().is_empty() {
            machine.bus.write(0x003A, 0x04);
            acia_redirected = true;
        }
        if machine.bus.acia().output().windows(2).any(|w| w == b"A>") {
            a_prompt_seen = true;
            break;
        }
    }

    // REQ-M4-4: SIM cold-init completed — A> prompt appeared without hang
    assert!(
        a_prompt_seen,
        "REQ-M4-4: 'A>' not seen after {} cycles; output: {:?}",
        MAX_CYCLES,
        String::from_utf8_lossy(machine.bus.acia().output())
    );

    // mc-hpg: real firmware's Multi-I/O keyboard probe (bios_multi.asm KBD_PROBE)
    // must complete cleanly. All three of its failure messages ("NOT FOUND",
    // "WRITE TIMEOUT", "READ TIMEOUT") contain "VT82C42"; the success message
    // ("KBD: INITIALIZED.") does not. A synthetic bus-poke test cannot catch a
    // busy/data-pending status-bit regression the way driving real firmware can.
    let transcript = String::from_utf8_lossy(machine.bus.acia().output());
    assert!(
        !transcript.contains("VT82C42"),
        "Multi-I/O keyboard probe reported an error during real-firmware boot: {transcript}"
    );

    // REQ-M4-3: far-call returned cleanly — task-0 DOS main was copied ($B800 non-zero)
    assert_ne!(
        machine.bus.phys_read(0xB800), 0,
        "REQ-M4-3: physical $B800 must be non-zero (task-0 DOS main copied)"
    );

    // REQ-M4-3: task-1 driver was copied ($10000 non-zero) — far-call target exists
    assert_ne!(
        machine.bus.phys_read(0x10000), 0,
        "REQ-M4-3: physical $10000 must be non-zero (task-1 driver copied)"
    );
}
