use crate::bus::Bus;
use crate::config::Config;
use crate::cpu::Cpu;
use crate::disk::DiskImage;
use crate::rom::Rom;

/// Top-level machine: owns the CPU, bus, and all peripherals.
pub struct Machine {
    cpu: Cpu,
    bus: Bus,
}

impl Machine {
    pub fn new(cfg: Config) -> Self {
        let rom = match &cfg.rom_hex {
            Some(path) => Rom::load_hex(path, cfg.rom_bank.clone())
                .unwrap_or_else(|_| Rom::blank(cfg.rom_bank.clone())),
            None => Rom::blank(cfg.rom_bank.clone()),
        };
        let disk = cfg.disk_image.as_ref().and_then(|p| DiskImage::load(p).ok());
        let bus = Bus::new(&cfg, rom, disk);
        let cpu = Cpu::new();
        Machine { cpu, bus }
    }

    /// Apply the RESET vector and start the main emulation loop.
    /// Stub: full loop implemented in WI-M1.
    pub fn run(&mut self) {
        {
            let bus = &mut self.bus;
            self.cpu.reset(|addr| bus.read(addr));
        }
        // Main loop placeholder — implemented in WI-M1.
    }
}
