// Gate test for WI-M3: XT-IDE controller, disk image, VIDEO ROM 60-sector boot.
//
// Acceptance:
//   1. VIDEO bank reset vector = $F000.
//   2. Probe writes $FF/$00 to $E300–$E330: no crash; disk LBA 0 unchanged.
//   3. SET FEATURES $EF: BSY clears; DRQ and ERR absent.
//   4. 60 sector reads succeed; sector transfer counter reaches 60.
//   5. CPU PC == $0800 after 60th transfer.

use emulator::bus::Bus;
use emulator::config::{Config, RomBank};
use emulator::cpu::Cpu;
use emulator::disk::DiskImage;
use emulator::emulator::Machine;
use emulator::rom::Rom;

// Minimal 6502 VIDEO ROM boot loader assembled at $F000.
//
// Reads 60 sectors sequentially from XT-IDE LBA 0–59 into RAM at $0800,
// storing 512 bytes per sector via ($00),Y indirect indexed addressing.
// After all sectors are loaded, jumps to $0800.
//
// Zero-page layout:
//   $00/$01 — load address (lo/hi), starts at $00/$08 ($0800)
//   $02     — sector counter, starts at 0, max 60
//
// Reset vector at ROM offsets $FFC/$FFD = $00/$F0 → $F000.
#[rustfmt::skip]
fn build_video_rom() -> Rom {
    let mut video = [0xFFu8; 4096];

    // Boot loader: init load address and sector counter
    let boot: &[u8] = &[
        // $F000: init load ptr and sector counter
        0xA9, 0x00, 0x85, 0x00,             // LDA #$00 ; STA $00  (lo = 0)
        0xA9, 0x08, 0x85, 0x01,             // LDA #$08 ; STA $01  (hi = $08)
        0xA9, 0x00, 0x85, 0x02,             // LDA #$00 ; STA $02  (sector = 0)
        // $F00C: next_sector — compare and branch when done
        0xA5, 0x02, 0xC9, 0x3C, 0xF0, 0x3E,// LDA $02 ; CMP #60 ; BEQ $F050
        // issue READ SECTORS command
        0xA5, 0x02, 0x8D, 0x03, 0xE3,       // LDA $02 ; STA $E303 (LBA[7:0])
        0xA9, 0x00, 0x8D, 0x04, 0xE3,       // LDA #$00 ; STA $E304
        0x8D, 0x05, 0xE3,                    // STA $E305
        0x8D, 0x06, 0xE3,                    // STA $E306
        0xA9, 0x01, 0x8D, 0x02, 0xE3,       // LDA #1 ; STA $E302 (sector count)
        0xA9, 0x20, 0x8D, 0x07, 0xE3,       // LDA #$20 ; STA $E307 (READ SECTORS)
        // $F02C: wait_drq — poll until DRQ set
        0xAD, 0x07, 0xE3,                    // LDA $E307 (status)
        0x29, 0x08,                          // AND #$08 (DRQ bit)
        0xF0, 0xF9,                          // BEQ $F02C
        // $F033: read lo 256 bytes
        0xA0, 0x00,                          // LDY #$00
        // $F035: read_lo loop
        0xAD, 0x00, 0xE3,                    // LDA $E300 (data port)
        0x91, 0x00,                          // STA ($00),Y
        0xC8,                                // INY
        0xD0, 0xF8,                          // BNE $F035
        // $F03D: advance to hi half page
        0xE6, 0x01,                          // INC $01
        // $F03F: read hi 256 bytes
        0xA0, 0x00,                          // LDY #$00
        // $F041: read_hi loop
        0xAD, 0x00, 0xE3,                    // LDA $E300 (data port)
        0x91, 0x00,                          // STA ($00),Y
        0xC8,                                // INY
        0xD0, 0xF8,                          // BNE $F041
        // $F049: advance load ptr and sector counter; repeat
        0xE6, 0x01,                          // INC $01
        0xE6, 0x02,                          // INC $02
        0x4C, 0x0C, 0xF0,                    // JMP $F00C
        // $F050: done — jump to loaded code
        0x4C, 0x00, 0x08,                    // JMP $0800
    ];
    video[..boot.len()].copy_from_slice(boot);

    // Reset vector: $F000 in little-endian at $FFFC–$FFFD (ROM offsets $FFC–$FFD)
    video[0xFFC] = 0x00;
    video[0xFFD] = 0xF0;

    Rom::from_banks([0xFFu8; 4096], video, RomBank::Video)
}

#[test]
fn m3_video_boot_and_60_sectors() {
    // === Hardware unit tests on a standalone Bus ===
    let cfg = Config::default();
    let mut bus = Bus::new(&cfg, build_video_rom(), Some(DiskImage::blank(100)));

    // 1. VIDEO bank reset vector must be $F000
    let reset_lo = bus.read(0xFFFC) as u16;
    let reset_hi = bus.read(0xFFFD) as u16;
    assert_eq!(
        (reset_hi << 8) | reset_lo, 0xF000,
        "VIDEO bank reset vector must be $F000"
    );

    // 2. SET FEATURES $EF: BSY clears; DRQ and ERR absent
    bus.write(0xE307, 0xEF);
    let st = bus.read(0xE307);
    assert_eq!(st & 0x80, 0, "BSY must not be set after SET FEATURES");
    assert_eq!(st & 0x08, 0, "DRQ must not be set after SET FEATURES");
    assert_eq!(st & 0x01, 0, "ERR must not be set after SET FEATURES");

    // 3. Probe writes $FF/$00 to $E300–$E330 must not crash (BR-5)
    for addr in 0xE300u16..=0xE330 {
        bus.write(addr, 0xFF);
        bus.write(addr, 0x00);
    }

    // 4. Disk LBA 0 unchanged after probe writes (sector data stays all-zero)
    bus.write(0xE302, 0x01); // sector count = 1
    bus.write(0xE303, 0x00);
    bus.write(0xE304, 0x00);
    bus.write(0xE305, 0x00);
    bus.write(0xE306, 0x00);
    bus.write(0xE307, 0x20); // READ SECTORS
    let lba0: Vec<u8> = (0..512).map(|_| bus.read(0xE300)).collect();
    assert!(
        lba0.iter().all(|&b| b == 0),
        "LBA 0 disk data must be unchanged (all zeros) after probe writes"
    );

    // === Full emulation: VIDEO ROM boot loads 60 sectors then jumps to $0800 ===
    let mut machine = Machine {
        cpu: Cpu::new(),
        bus: Bus::new(&Config::default(), build_video_rom(), Some(DiskImage::blank(100))),
    };
    {
        let bus = &mut machine.bus;
        machine.cpu.reset(|addr| bus.read(addr));
    }
    assert_eq!(machine.cpu.pc, 0xF000, "PC after reset must be $F000");

    const MAX_CYCLES: u64 = 2_000_000;
    let mut total: u64 = 0;
    loop {
        let cycles = machine.step_one() as u64;
        total += cycles;
        if machine.cpu.pc == 0x0800 {
            break;
        }
        assert!(
            total < MAX_CYCLES,
            "timeout after {} cycles: PC=${:04X}; sector count={}",
            total, machine.cpu.pc, machine.bus.read(0x0002)
        );
    }

    // 5. Sector transfer counter must reach 60
    let sector_count = machine.bus.read(0x0002);
    assert_eq!(sector_count, 60, "sector transfer count must reach 60");

    // 6. CPU PC must be $0800 after the 60th transfer
    assert_eq!(machine.cpu.pc, 0x0800, "PC must be $0800 after final sector transfer");
}
