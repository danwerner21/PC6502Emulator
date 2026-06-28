use crate::bus::Bus;
use crate::config::Config;
use crate::cpu::Cpu;
use crate::disk::DiskImage;
use crate::rom::Rom;

/// Top-level machine: owns the CPU, bus, and all peripherals.
pub struct Machine {
    pub cpu: Cpu,
    pub bus: Bus,
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

    /// Execute one instruction.  Returns cycles consumed.
    pub fn step_one(&mut self) -> u32 {
        // Safety: read and write callbacks are called sequentially by the CPU,
        // never concurrently, so aliasing the bus pointer is sound.
        let bus: *mut Bus = &mut self.bus;
        let cpu = &mut self.cpu;
        cpu.step(
            |addr| unsafe { (*bus).read(addr) },
            |addr, val| unsafe { (*bus).write(addr, val) },
        )
    }

    /// Run with RESET and drain ACIA output to stdout in a loop.
    pub fn run(&mut self) {
        {
            let bus = &mut self.bus;
            self.cpu.reset(|addr| bus.read(addr));
        }
        loop {
            self.step_one();
            // Drain buffered serial output to stdout
            let out = self.bus.acia_mut().drain_output();
            if !out.is_empty() {
                use std::io::Write;
                let _ = std::io::stdout().write_all(&out);
                let _ = std::io::stdout().flush();
            }
        }
    }

    /// Run up to `max_cycles` after RESET, returning total cycles executed.
    pub fn run_until_cycles(&mut self, max_cycles: u64) -> u64 {
        {
            let bus = &mut self.bus;
            self.cpu.reset(|addr| bus.read(addr));
        }
        let mut total: u64 = 0;
        while total < max_cycles {
            total += self.step_one() as u64;
        }
        total
    }
}
