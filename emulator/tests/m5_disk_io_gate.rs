// Gate test for WI-M5: DOS/65 disk read/write, directory listing.
//
// Acceptance (REQ-M5):
//   1. WRITE SECTORS ($30): 512-byte transfer via data port; read-back identical.
//   2. Bad-sector injection: ERR bit set on READ/WRITE to injected LBA; DRQ clear.
//   3. Drive-B isolation: write to LBA $4100 leaves LBA 0 unchanged.
//   4. IDENTIFY: full 512-byte response; model string at bytes 54–93.
//   5. CPU-driven write roundtrip: synthetic ROM writes LBA 1 via WRITE SECTORS;
//      host read-back confirms data committed to disk image.

use emulator::bus::Bus;
use emulator::config::{Config, RomBank};
use emulator::cpu::Cpu;
use emulator::disk::DiskImage;
use emulator::emulator::Machine;
use emulator::rom::Rom;

// Synthetic ROM that writes 512 bytes of $42 to LBA 1 via WRITE SECTORS,
// then hangs. Used to test the CPU-driven write path end-to-end.
//
// Layout (all at $F000):
//   $F000: set LBA=1, sector_count=1, issue WRITE SECTORS ($30)
//   $F01A: poll DRQ until set
//   $F021: write 512 bytes of $42 to data port ($E300)
//   $F030: JMP $F030  — hang
//
// Reset vector: $FFFC/$FFFD = $00/$F0 → $F000.
#[rustfmt::skip]
fn build_write_rom() -> Rom {
    let mut base = [0xFFu8; 4096];

    let boot: &[u8] = &[
        // $F000: set LBA registers for LBA 1
        0xA9, 0x01, 0x8D, 0x03, 0xE3,  // LDA #$01 ; STA $E303  (LBA[7:0] = 1)
        0xA9, 0x00, 0x8D, 0x04, 0xE3,  // LDA #$00 ; STA $E304  (LBA[15:8] = 0)
        0x8D, 0x05, 0xE3,              // STA $E305  (LBA[23:16] = 0)
        0x8D, 0x06, 0xE3,              // STA $E306  (LBA[27:24] = 0)
        0xA9, 0x01, 0x8D, 0x02, 0xE3,  // LDA #$01 ; STA $E302  (sector count = 1)
        0xA9, 0x30, 0x8D, 0x07, 0xE3,  // LDA #$30 ; STA $E307  (WRITE SECTORS command)

        // $F01A: poll status until DRQ set
        0xAD, 0x07, 0xE3,              // LDA $E307
        0x29, 0x08,                    // AND #$08  (DRQ bit)
        0xF0, 0xF9,                    // BEQ $F01A

        // $F021: write 512 bytes ($42) via data port: 2 pages × 256 bytes
        0xA9, 0x42,                    // LDA #$42  (pattern byte)
        0xA2, 0x02,                    // LDX #$02  (2 pages)
        0xA0, 0x00,                    // LDY #$00
        // $F027: loop_w
        0x8D, 0x00, 0xE3,              // STA $E300  (write data byte)
        0xC8,                          // INY
        0xD0, 0xFA,                    // BNE $F027  (256 per page)
        0xCA,                          // DEX
        0xD0, 0xF5,                    // BNE $F025  (two pages total)

        // $F030: hang
        0x4C, 0x30, 0xF0,              // JMP $F030
    ];
    base[..boot.len()].copy_from_slice(boot);

    // Reset vector at ROM offsets $FFC/$FFD = $F000 (little-endian)
    base[0xFFC] = 0x00;
    base[0xFFD] = 0xF0;

    Rom::from_banks(base, [0xFFu8; 4096], RomBank::Base)
}

#[test]
fn m5_disk_io_gate() {
    // ===================================================================
    // Section 1: XT-IDE unit tests on a standalone Bus
    // ===================================================================

    let cfg = Config::default();
    let mut bus = Bus::new(&cfg, Rom::blank(RomBank::Base), Some(DiskImage::blank(20000)));

    // --- 1a. WRITE SECTORS ($30): roundtrip write→read, all bytes match ---
    bus.write(0xE303, 3);    // LBA[7:0] = 3
    bus.write(0xE304, 0);
    bus.write(0xE305, 0);
    bus.write(0xE306, 0);
    bus.write(0xE302, 1);    // sector count = 1
    bus.write(0xE307, 0x30); // WRITE SECTORS command

    // DRQ must be set — controller is waiting for data
    let st = bus.read(0xE307);
    assert_eq!(st & 0x08, 0x08, "DRQ must be set after WRITE SECTORS command");
    assert_eq!(st & 0x01, 0x00, "ERR must be clear after WRITE SECTORS command");

    // Feed 512 bytes of $5A
    for _ in 0..512 {
        bus.write(0xE300, 0x5A);
    }

    // After 512 bytes, DRQ must clear and no error
    let st = bus.read(0xE307);
    assert_eq!(st & 0x08, 0x00, "DRQ must clear after all 512 data bytes written");
    assert_eq!(st & 0x01, 0x00, "ERR must not be set after successful write");

    // Read back via READ SECTORS to LBA 3
    bus.write(0xE303, 3);
    bus.write(0xE304, 0);
    bus.write(0xE305, 0);
    bus.write(0xE306, 0);
    bus.write(0xE302, 1);
    bus.write(0xE307, 0x20); // READ SECTORS
    let readback: Vec<u8> = (0..512).map(|_| bus.read(0xE300)).collect();
    assert!(
        readback.iter().all(|&b| b == 0x5A),
        "read-back of written sector must be all $5A; got first byte {:#04x}",
        readback[0]
    );

    // --- 1b. Bad-sector injection: READ to bad LBA → ERR, DRQ clear ---
    bus.xt_ide_mut().inject_bad_sector(7);

    bus.write(0xE303, 7);    // LBA = 7 (bad)
    bus.write(0xE304, 0);
    bus.write(0xE305, 0);
    bus.write(0xE306, 0);
    bus.write(0xE302, 1);
    bus.write(0xE307, 0x20); // READ SECTORS
    let st = bus.read(0xE307);
    assert_ne!(st & 0x01, 0, "ERR must be set for READ to bad-sector LBA");
    assert_eq!(st & 0x08, 0, "DRQ must be clear for READ to bad-sector LBA");

    // --- 1c. Bad-sector injection: WRITE to bad LBA → ERR, DRQ clear ---
    bus.write(0xE303, 7);
    bus.write(0xE304, 0);
    bus.write(0xE305, 0);
    bus.write(0xE306, 0);
    bus.write(0xE302, 1);
    bus.write(0xE307, 0x30); // WRITE SECTORS
    let st = bus.read(0xE307);
    assert_ne!(st & 0x01, 0, "ERR must be set for WRITE to bad-sector LBA");
    assert_eq!(st & 0x08, 0, "DRQ must be clear for WRITE to bad-sector LBA");

    // Clear bad sector — subsequent access should succeed
    bus.xt_ide_mut().clear_bad_sector(7);
    bus.write(0xE303, 7);
    bus.write(0xE304, 0);
    bus.write(0xE305, 0);
    bus.write(0xE306, 0);
    bus.write(0xE302, 1);
    bus.write(0xE307, 0x20); // READ SECTORS
    let st = bus.read(0xE307);
    assert_eq!(st & 0x01, 0, "ERR must be clear after bad-sector cleared");
    assert_ne!(st & 0x08, 0, "DRQ must be set after bad-sector cleared");
    // drain the sector
    for _ in 0..512 { let _ = bus.read(0xE300); }

    // --- 1d. Drive-B isolation: write LBA $4100; LBA 0 must be unchanged ---
    // Write $BB to LBA $4100 (B-drive area per REQ-M5-6)
    bus.write(0xE303, 0x00); // LBA[7:0]
    bus.write(0xE304, 0x41); // LBA[15:8]  → LBA = 0x00_4100
    bus.write(0xE305, 0x00);
    bus.write(0xE306, 0x00);
    bus.write(0xE302, 1);
    bus.write(0xE307, 0x30); // WRITE SECTORS
    for _ in 0..512 { bus.write(0xE300, 0xBB); }

    // Read LBA 0 — must still be all zeros (blank disk, never written)
    bus.write(0xE303, 0);
    bus.write(0xE304, 0);
    bus.write(0xE305, 0);
    bus.write(0xE306, 0);
    bus.write(0xE302, 1);
    bus.write(0xE307, 0x20); // READ SECTORS
    let lba0: Vec<u8> = (0..512).map(|_| bus.read(0xE300)).collect();
    assert!(
        lba0.iter().all(|&b| b == 0),
        "LBA 0 must be unchanged after B-range (LBA $4100) write"
    );

    // --- 1e. IDENTIFY: model string at bytes 54–93 ---
    bus.write(0xE307, 0xEC); // IDENTIFY
    let identify: Vec<u8> = (0..512).map(|_| bus.read(0xE300)).collect();
    assert_eq!(
        &identify[54..94],
        b"PC6502 DISK                             ",
        "IDENTIFY model string must be 'PC6502 DISK' (space-padded to 40 bytes)"
    );
    // LBA-capable flag: word 49 bit 9 = byte 99 bit 1
    assert_ne!(identify[99] & 0x02, 0, "IDENTIFY must report LBA support");

    // ===================================================================
    // Section 2: CPU-driven WRITE SECTORS via synthetic ROM
    // ===================================================================
    //
    // Synthetic ROM writes 512 bytes of $42 to LBA 1 via WRITE SECTORS,
    // then hangs at $F030. Test verifies the sector was committed by
    // reading it back directly after the CPU halts.

    let disk = DiskImage::blank(20000);
    let mut machine = Machine {
        cpu: Cpu::new(),
        bus: Bus::new(&Config::default(), build_write_rom(), Some(disk)),
    };
    {
        let bus = &mut machine.bus;
        machine.cpu.reset(|addr| bus.read(addr));
    }
    assert_eq!(machine.cpu.pc, 0xF000, "PC after reset must be $F000");

    const MAX_CYCLES: u64 = 500_000;
    let mut total: u64 = 0;
    loop {
        total += machine.step_one() as u64;
        if machine.cpu.pc == 0xF030 {
            break;
        }
        assert!(
            total < MAX_CYCLES,
            "timeout after {} cycles: PC=${:04X}",
            total,
            machine.cpu.pc
        );
    }

    // Read back LBA 1 via direct bus commands (no CPU involvement)
    machine.bus.write(0xE303, 1);
    machine.bus.write(0xE304, 0);
    machine.bus.write(0xE305, 0);
    machine.bus.write(0xE306, 0);
    machine.bus.write(0xE302, 1);
    machine.bus.write(0xE307, 0x20); // READ SECTORS
    let sector: Vec<u8> = (0..512).map(|_| machine.bus.read(0xE300)).collect();
    assert!(
        sector.iter().all(|&b| b == 0x42),
        "CPU-written sector at LBA 1 must read back as all $42; first byte was {:#04x}",
        sector[0]
    );
}
