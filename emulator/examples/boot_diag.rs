use emulator::{bus::Bus, config::{Config, RomBank}, cpu::Cpu, disk::DiskImage, emulator::Machine, rom::Rom};

fn main() {
    let rom_path = std::env::var("PC6502_ROM_HEX").expect("set PC6502_ROM_HEX");
    let disk_path = std::env::var("PC6502_DISK_IMG").expect("set PC6502_DISK_IMG");

    let rom = Rom::load_hex(&rom_path, RomBank::Video).unwrap();
    let disk = DiskImage::load(&disk_path).unwrap();
    let mut machine = Machine { cpu: Cpu::new(), bus: Bus::new(&Config::default(), rom, Some(disk)) };
    { let b = &mut machine.bus; machine.cpu.reset(|a| b.read(a)); }

    let mut total: u64 = 0;
    let mut last_acia = 0usize;
    let mut last_milestone = 0u64;
    let mut acia_redirected = false;

    loop {
        total += machine.step_one() as u64;

        if !acia_redirected && !machine.bus.acia().output().is_empty() {
            machine.bus.write(0x003A, 0x04);
            acia_redirected = true;
        }

        let acia_len = machine.bus.acia().output().len();
        if acia_len > last_acia {
            let out = machine.bus.acia().output();
            eprintln!("cycle={:>10} PC=${:04X} new ACIA={:?}", total, machine.cpu.pc,
                String::from_utf8_lossy(&out[last_acia..]));
            last_acia = acia_len;
        }
        if total - last_milestone >= 10_000_000 {
            eprintln!("cycle={:>10} PC=${:04X} ACIA_len={} SP=${:02X}",
                total, machine.cpu.pc, acia_len, machine.cpu.sp);
            last_milestone = total;
        }
        if machine.bus.acia().output().windows(2).any(|w| w == b"A>") {
            eprintln!("SUCCESS at cycle {}", total);
            return;
        }
        if total >= 300_000_000 { eprintln!("TIMEOUT 300M cycles PC=${:04X}", machine.cpu.pc); return; }
    }
}
