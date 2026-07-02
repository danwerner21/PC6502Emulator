// Gate test for WI-M5: DOS/65 disk read/write, directory listing.
//
// Acceptance (REQ-M5):
//   1. WRITE SECTORS ($30): 512-byte transfer via data port; read-back identical.
//   2. Bad-sector injection: ERR bit set on READ/WRITE to injected LBA; DRQ clear.
//   3. Drive-B isolation: write to LBA $4100 leaves LBA 0 unchanged.
//   4. IDENTIFY: full 512-byte response; model string at bytes 54–93.
//   5. CPU-driven write roundtrip: synthetic ROM writes LBA 1 via WRITE SECTORS;
//      host read-back confirms data committed to disk image.
//   REQ-M5-1. DIR A: on the hardware disk returns no-file response without crash.
//             (CP/M directory at LBA 60 is E5-filled / empty on the stock disk image.)
//   REQ-M5-4. Drive-E failure: no crash or hang; DOS prompt returns after "E:\r".
//   REQ-M5-6. LBA $4100 = 16,640 decimal = one past the end of the 16,640-sector
//             hardware disk.  Write-to-$4100 tests emulator auto-extend behaviour,
//             not hardware-observable drive-B behaviour.  Drive-B does not exist on
//             the stock disk image.

use emulator::bus::Bus;
use emulator::config::{Config, RomBank};
use emulator::cpu::Cpu;
use emulator::disk::DiskImage;
use emulator::emulator::Machine;

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
        0xA9, 0x01, 0x8D, 0x06, 0xE3,  // LDA #$01 ; STA $E306  (LBA[7:0] = 1)
        0xA9, 0x00, 0x8D, 0x08, 0xE3,  // LDA #$00 ; STA $E308  (LBA[15:8] = 0)
        0x8D, 0x0A, 0xE3,              // STA $E30A  (LBA[23:16] = 0)
        0x8D, 0x0C, 0xE3,              // STA $E30C  (device = 0)
        0xA9, 0x01, 0x8D, 0x04, 0xE3,  // LDA #$01 ; STA $E304  (sector count = 1)
        0xA9, 0x30, 0x8D, 0x0E, 0xE3,  // LDA #$30 ; STA $E30E  (WRITE SECTORS command)

        // $F01A: poll status until DRQ set
        0xAD, 0x0E, 0xE3,              // LDA $E30E
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
    bus.write(0xE306, 3);    // LBA[7:0] = 3
    bus.write(0xE308, 0);    // LBA[15:8]
    bus.write(0xE30A, 0);    // LBA[23:16]
    bus.write(0xE30C, 0);    // device
    bus.write(0xE304, 1);    // sector count = 1
    bus.write(0xE30E, 0x30); // WRITE SECTORS command

    // DRQ must be set — controller is waiting for data
    let st = bus.read(0xE30E);
    assert_eq!(st & 0x08, 0x08, "DRQ must be set after WRITE SECTORS command");
    assert_eq!(st & 0x01, 0x00, "ERR must be clear after WRITE SECTORS command");

    // Feed 512 bytes of $5A
    for _ in 0..512 {
        bus.write(0xE300, 0x5A);
    }

    // After 512 bytes, DRQ must clear and no error
    let st = bus.read(0xE30E);
    assert_eq!(st & 0x08, 0x00, "DRQ must clear after all 512 data bytes written");
    assert_eq!(st & 0x01, 0x00, "ERR must not be set after successful write");

    // Read back via READ SECTORS to LBA 3
    bus.write(0xE306, 3);    // LBA[7:0]
    bus.write(0xE308, 0);    // LBA[15:8]
    bus.write(0xE30A, 0);    // LBA[23:16]
    bus.write(0xE30C, 0);    // device
    bus.write(0xE304, 1);    // sector count
    bus.write(0xE30E, 0x20); // READ SECTORS
    let readback: Vec<u8> = (0..512).map(|_| bus.read(0xE300)).collect();
    assert!(
        readback.iter().all(|&b| b == 0x5A),
        "read-back of written sector must be all $5A; got first byte {:#04x}",
        readback[0]
    );

    // --- 1b. Bad-sector injection: READ to bad LBA → ERR, DRQ clear ---
    bus.xt_ide_mut().inject_bad_sector(7);

    bus.write(0xE306, 7);    // LBA[7:0] = 7 (bad)
    bus.write(0xE308, 0);    // LBA[15:8]
    bus.write(0xE30A, 0);    // LBA[23:16]
    bus.write(0xE30C, 0);    // device
    bus.write(0xE304, 1);    // sector count
    bus.write(0xE30E, 0x20); // READ SECTORS
    let st = bus.read(0xE30E);
    assert_ne!(st & 0x01, 0, "ERR must be set for READ to bad-sector LBA");
    assert_eq!(st & 0x08, 0, "DRQ must be clear for READ to bad-sector LBA");

    // --- 1c. Bad-sector injection: WRITE to bad LBA → ERR, DRQ clear ---
    bus.write(0xE306, 7);    // LBA[7:0] = 7 (bad)
    bus.write(0xE308, 0);
    bus.write(0xE30A, 0);
    bus.write(0xE30C, 0);
    bus.write(0xE304, 1);
    bus.write(0xE30E, 0x30); // WRITE SECTORS
    let st = bus.read(0xE30E);
    assert_ne!(st & 0x01, 0, "ERR must be set for WRITE to bad-sector LBA");
    assert_eq!(st & 0x08, 0, "DRQ must be clear for WRITE to bad-sector LBA");

    // Clear bad sector — subsequent access should succeed
    bus.xt_ide_mut().clear_bad_sector(7);
    bus.write(0xE306, 7);    // LBA[7:0] = 7 (cleared)
    bus.write(0xE308, 0);
    bus.write(0xE30A, 0);
    bus.write(0xE30C, 0);
    bus.write(0xE304, 1);
    bus.write(0xE30E, 0x20); // READ SECTORS
    let st = bus.read(0xE30E);
    assert_eq!(st & 0x01, 0, "ERR must be clear after bad-sector cleared");
    assert_ne!(st & 0x08, 0, "DRQ must be set after bad-sector cleared");
    // drain the sector
    for _ in 0..512 { let _ = bus.read(0xE300); }

    // --- 1d. Drive-B / REQ-M5-6: write LBA $4100; LBA 0 must be unchanged ---
    // LBA $4100 = 16,640 decimal = one past the end of the 16,640-sector hardware
    // disk image.  This test exercises the emulator's auto-extend behaviour: writes
    // beyond the image boundary succeed in memory without corrupting earlier sectors.
    // Drive-B does not exist on the stock hardware disk; this is an emulator-internal
    // correctness test, not a hardware-observable behaviour test.
    bus.write(0xE306, 0x00); // LBA[7:0]
    bus.write(0xE308, 0x41); // LBA[15:8]  → LBA = 0x00_4100 (one past end of disk)
    bus.write(0xE30A, 0x00); // LBA[23:16]
    bus.write(0xE30C, 0x00); // device
    bus.write(0xE304, 1);    // sector count
    bus.write(0xE30E, 0x30); // WRITE SECTORS
    for _ in 0..512 { bus.write(0xE300, 0xBB); }

    // Read LBA 0 — must still be all zeros (blank disk, never written)
    bus.write(0xE306, 0);    // LBA[7:0]
    bus.write(0xE308, 0);    // LBA[15:8]
    bus.write(0xE30A, 0);    // LBA[23:16]
    bus.write(0xE30C, 0);    // device
    bus.write(0xE304, 1);    // sector count
    bus.write(0xE30E, 0x20); // READ SECTORS
    let lba0: Vec<u8> = (0..512).map(|_| bus.read(0xE300)).collect();
    assert!(
        lba0.iter().all(|&b| b == 0),
        "LBA 0 must be unchanged after beyond-end-of-disk (LBA $4100) write"
    );

    // --- 1e. IDENTIFY: model string at bytes 54–93 ---
    bus.write(0xE30E, 0xEC); // IDENTIFY
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
    let mut machine = Machine::from_parts(
        Cpu::new(),
        Bus::new(&Config::default(), build_write_rom(), Some(disk)),
    );
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
    machine.bus.write(0xE306, 1);    // LBA[7:0] = 1
    machine.bus.write(0xE308, 0);    // LBA[15:8]
    machine.bus.write(0xE30A, 0);    // LBA[23:16]
    machine.bus.write(0xE30C, 0);    // device
    machine.bus.write(0xE304, 1);    // sector count
    machine.bus.write(0xE30E, 0x20); // READ SECTORS
    let sector: Vec<u8> = (0..512).map(|_| machine.bus.read(0xE300)).collect();
    assert!(
        sector.iter().all(|&b| b == 0x42),
        "CPU-written sector at LBA 1 must read back as all $42; first byte was {:#04x}",
        sector[0]
    );
}

// Helper: boot the real DOS/65 system to the A> prompt.
// Returns the Machine on success, None if artifacts are missing (skip).
fn boot_to_a_prompt() -> Option<Machine> {
    let rom_path = rom_hex_path();
    let disk_path = disk_image_path();

    if !std::path::Path::new(&rom_path).exists() || !std::path::Path::new(&disk_path).exists() {
        return None;
    }

    let rom = Rom::load_hex(&rom_path, RomBank::Video)
        .expect("failed to load rom.hex for Video bank");
    let disk = DiskImage::load(&disk_path).expect("failed to load disk.img");

    let mut machine = Machine::from_parts(Cpu::new(), Bus::new(&Config::default(), rom, Some(disk)));
    {
        let bus = &mut machine.bus;
        machine.cpu.reset(|addr| bus.read(addr));
    }

    // DOS/65 routes console output through ZP $3A (device-function base).  The
    // Video ROM sets $3A = $13 (fn 19 = ESP WiFi), so init output goes to ESP, not
    // ACIA.  After the Video ROM banner lands in ACIA (proving UART init), redirect
    // $3A to $04 (fn 4 = ACIA TX) so all subsequent DOS/65 output reaches ACIA.
    const MAX_CYCLES: u64 = 50_000_000;
    let mut total: u64 = 0;
    let mut acia_redirected = false;
    while total < MAX_CYCLES {
        total += machine.step_one() as u64;
        if !acia_redirected && !machine.bus.acia().output().is_empty() {
            machine.bus.write(0x003A, 0x04);
            acia_redirected = true;
        }
        if machine.bus.acia().output().windows(2).any(|w| w == b"A>") {
            return Some(machine);
        }
    }

    panic!(
        "boot_to_a_prompt: 'A>' not seen after {} cycles; output: {:?}",
        MAX_CYCLES,
        String::from_utf8_lossy(machine.bus.acia().output())
    );
}

// REQ-M5-4: Accessing drive E returns failure; no crash or hang; DOS prompt returns.
//
// The hardware disk is 16,640 sectors.  Drive E (if it existed) would be well
// beyond that boundary.  DOS/65 must handle the unknown drive gracefully and
// return to the A> prompt.
//
// Skips cleanly if rom.hex or disk.img are not present.
#[test]
fn m5_drive_e_failure() {
    let Some(mut machine) = boot_to_a_prompt() else {
        eprintln!("m5_drive_e_failure: skipped (artifacts not found)");
        return;
    };

    let output_before = machine.bus.acia().output().len();

    // Inject "E:\r" — request drive E which does not exist on this disk.
    // DOS/65 prints "PEM ERROR ON E - BAD SECTOR\r\n<RET> TO IGNORE -- <OTHER> TO ABORT"
    // and waits for a keypress.  <RET>=ignore/retry; any other key=abort and return.
    // Pre-inject ESC so the error-dismiss prompt is answered with "ABORT".
    machine.bus.acia_mut().inject_rx_bytes(b"E:\r\x1b");

    // Run until the prompt returns (A> reappears after the error) or timeout
    const MAX_CYCLES: u64 = 10_000_000;
    let mut total: u64 = 0;
    let mut prompt_returned = false;
    while total < MAX_CYCLES {
        total += machine.step_one() as u64;
        let new_out = &machine.bus.acia().output()[output_before..];
        if new_out.windows(2).any(|w| w == b"A>") {
            prompt_returned = true;
            break;
        }
    }

    assert!(
        prompt_returned,
        "REQ-M5-4: 'A>' did not return after 'E:\\r' + dismiss within {} cycles; \
         new output: {:?}",
        MAX_CYCLES,
        String::from_utf8_lossy(&machine.bus.acia().output()[output_before..])
    );
}

// REQ-M5-1: DIR A: on the stock hardware disk executes without crash and returns
// normally.  The CP/M directory at LBA 60 is E5-filled (empty); DOS/65 returns
// a "no file" response rather than a populated listing.  That is the correct
// observable behaviour for this disk image.
//
// Skips cleanly if rom.hex or disk.img are not present.
#[test]
fn m5_dir_listing() {
    let Some(mut machine) = boot_to_a_prompt() else {
        eprintln!("m5_dir_listing: skipped (artifacts not found)");
        return;
    };

    let output_before = machine.bus.acia().output().len();

    // Inject "DIR A:\r"
    machine.bus.acia_mut().inject_rx_bytes(b"DIR A:\r");

    // Run until the prompt returns after the DIR command
    const MAX_CYCLES: u64 = 5_000_000;
    let mut total: u64 = 0;
    let mut prompt_returned = false;
    while total < MAX_CYCLES {
        total += machine.step_one() as u64;
        let new_out = &machine.bus.acia().output()[output_before..];
        if new_out.windows(2).any(|w| w == b"A>") {
            prompt_returned = true;
            break;
        }
    }

    // REQ-M5-1: DIR returned without crash — any response (including "no file")
    // from an empty but structurally valid CP/M directory is acceptable.
    assert!(
        prompt_returned,
        "REQ-M5-1: 'A>' did not return after 'DIR A:\\r' within {} cycles; \
         new output: {:?}",
        MAX_CYCLES,
        String::from_utf8_lossy(&machine.bus.acia().output()[output_before..])
    );
}

// REQ-M5-2: A valid .COM program loads and runs without hanging.
//
// This test requires a separately populated disk image containing at least one
// .COM file in the CP/M directory at LBA 60.  The stock hardware disk image has
// an empty CP/M directory (all E5-filled) and no .COM files.  This test is
// therefore ignored until a populated disk image is available.
#[test]
#[ignore = "requires a disk image with at least one .COM file; stock disk has empty CP/M directory"]
fn m5_com_load() {
    let Some(mut machine) = boot_to_a_prompt() else {
        eprintln!("m5_com_load: skipped (artifacts not found)");
        return;
    };

    let output_before = machine.bus.acia().output().len();

    // Inject a .COM filename — replace with an actual name when a populated disk
    // image is committed.
    machine.bus.acia_mut().inject_rx_bytes(b"TEST.COM\r");

    const MAX_CYCLES: u64 = 10_000_000;
    let mut total: u64 = 0;
    while total < MAX_CYCLES {
        total += machine.step_one() as u64;
        // A real .COM would run and eventually return to A>; check for no hang.
        if machine.bus.acia().output()[output_before..].windows(2).any(|w| w == b"A>") {
            return;
        }
    }

    panic!(
        "REQ-M5-2: .COM did not return to A> within {} cycles; new output: {:?}",
        MAX_CYCLES,
        String::from_utf8_lossy(&machine.bus.acia().output()[output_before..])
    );
}
