// Gate test for WI-M4: DOS/65 cold boot — task-switching, absent-device stubs, A> gate.
//
// Acceptance:
//   1. Absent-device stubs: CH375 ($E260-$E261), ESP ($E100-$E102), Multi-I/O ($E3FE-$E3FF)
//      return open_bus on reads; writes are discarded; kbd self-test ($AA→$55) works.
//   2. physical $B800-$D870 non-zero after task-0 copy.
//   3. physical $10000-$11FFF non-zero after task-1 copy via MMU task-1 map.
//   4. stdout contains "DOS/65" then "A>".
//   5. inject "A:\r"; echo appears; no hang.

use emulator::bus::Bus;
use emulator::config::{Config, RomBank};
use emulator::cpu::Cpu;
use emulator::emulator::Machine;
use emulator::rom::Rom;

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
    let open_bus = cfg.open_bus.value; // $EA
    let mut bus = Bus::new(&cfg, build_dos65_boot_rom(), None);

    // CH375 $E260-$E261: reads return open_bus; writes discarded
    assert_eq!(bus.read(0xE260), open_bus, "CH375 $E260 must return open_bus");
    assert_eq!(bus.read(0xE261), open_bus, "CH375 $E261 must return open_bus");
    bus.write(0xE260, 0xFF);
    bus.write(0xE261, 0xFF);
    assert_eq!(bus.read(0xE260), open_bus, "CH375 write must be discarded");

    // Dual ESP $E100-$E102: reads return open_bus; writes discarded
    assert_eq!(bus.read(0xE100), open_bus, "ESP $E100 must return open_bus");
    assert_eq!(bus.read(0xE102), open_bus, "ESP $E102 must return open_bus");
    bus.write(0xE100, 0xFF);
    assert_eq!(bus.read(0xE100), open_bus, "ESP write must be discarded");

    // Multi-I/O $E3FE-$E3FF: keyboard self-test $AA → $55; other reads return open_bus
    bus.write(0xE3FE, 0xAA); // self-test command on offset 0
    assert_eq!(bus.read(0xE3FE), 0x55, "Multi-I/O kbd self-test must return $55");
    assert_eq!(bus.read(0xE3FE), open_bus, "Multi-I/O subsequent read must return open_bus");
    assert_eq!(bus.read(0xE3FF), open_bus, "Multi-I/O $E3FF must return open_bus");

    // === Section 2: full emulation — synthetic DOS/65 boot ROM ===
    let mut machine = Machine {
        cpu: Cpu::new(),
        bus: Bus::new(&Config::default(), build_dos65_boot_rom(), None),
    };
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
