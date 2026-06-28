use crate::acia::Acia;
use crate::config::Config;
use crate::disk::DiskImage;
use crate::mmu::Mmu;
use crate::peripherals::Peripherals;
use crate::rom::Rom;
use crate::rtc::Rtc;
use crate::xt_ide::XtIde;

/// PC6502 system bus.
///
/// CPU address map (MMU disabled / identity task-0):
///   $0000–$DFFF  — 56 KiB SRAM (physical $00000–$0DFFF)
///   $E000–$EFFF  — I/O overlay (decoded below)
///   $F000–$FFFF  — ROM (4 KiB, write-protected)
///
/// Physical address space (20-bit, 1 MiB):
///   $00000–$7FFFF — 512 KiB SRAM
///   I/O and ROM overlap at CPU-visible addresses when MMU is disabled.
///
/// I/O sub-map ($E000–$EFFF):
///   $E100–$E102  — Dual ESP (absent device stub)
///   $E260–$E261  — CH375 (absent device stub)
///   $E300–$E30E  — XT-IDE
///   $E3FE–$E3FF  — Multi-I/O keyboard (absent device stub)
///   $EF84–$EF87  — ACIA 6551
///   $EF90–$EF9F  — RTC-72421/72423
///   $EFD0–$EFDF  — MMU edit window
///   $EFE0–$EFE7  — MMU control registers
///   All other $E000–$EFFF — open-bus (read) / discard (write)
pub struct Bus {
    ram: Vec<u8>,
    rom: Rom,
    acia: Acia,
    mmu: Mmu,
    xt_ide: XtIde,
    rtc: Rtc,
    peripherals: Peripherals,
    open_bus: u8,
}

impl Bus {
    pub fn new(cfg: &Config, rom: Rom, disk: Option<DiskImage>) -> Self {
        let open_bus = cfg.open_bus.value;
        Bus {
            ram: vec![0u8; 512 * 1024],
            rom,
            acia: Acia::new(cfg),
            mmu: Mmu::new(cfg),
            xt_ide: XtIde::new(disk, open_bus),
            rtc: Rtc::new(cfg),
            peripherals: Peripherals::new(open_bus),
            open_bus,
        }
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0xF000..=0xFFFF => self.rom.read(addr - 0xF000),
            0xE000..=0xEFFF => self.io_read(addr),
            _ => {
                let phys = self.translate_ram(addr);
                if phys < self.ram.len() {
                    self.ram[phys]
                } else {
                    self.open_bus
                }
            }
        }
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0xF000..=0xFFFF => {} // ROM write protection (BR-8)
            0xE000..=0xEFFF => self.io_write(addr, val),
            _ => {
                let phys = self.translate_ram(addr);
                if phys < self.ram.len() {
                    self.ram[phys] = val;
                }
            }
        }
    }

    fn translate_ram(&self, addr: u16) -> usize {
        // When MMU is enabled, translate through the active task's map.
        // Identity map: physical = logical for task-0 in CPU $0000–$DFFF.
        // Physical pages above $7F are holes (BR-9); those reads return open-bus
        // (handled by the caller checking against ram.len()).
        match self.mmu.translate_addr(addr) {
            Some(phys) => phys as usize,
            None => addr as usize,
        }
    }

    fn io_read(&mut self, addr: u16) -> u8 {
        let lo = (addr & 0xFF) as u8;
        match addr {
            // XT-IDE $E300–$E30E
            0xE300..=0xE30E => self.xt_ide.read((addr - 0xE300) as u8),
            // Probe tolerance: $E30F–$E330 are silently ignored on reads too
            0xE30F..=0xE330 => self.open_bus,
            // ESP $E100–$E102
            0xE100..=0xE102 => self.peripherals.esp_read(lo),
            // CH375 $E260–$E261
            0xE260..=0xE261 => self.peripherals.ch375_read(lo),
            // Multi-I/O $E3FE–$E3FF
            0xE3FE..=0xE3FF => self.peripherals.multiio_read((addr - 0xE3FE) as u8),
            // ACIA $EF84–$EF87
            0xEF84..=0xEF87 => self.acia.read((addr - 0xEF84) as u8),
            // RTC $EF90–$EF9F
            0xEF90..=0xEF9F => self.rtc.read((addr - 0xEF90) as u8),
            // MMU edit window $EFD0–$EFDF
            0xEFD0..=0xEFDF => self.mmu.io_read((addr - 0xEFD0) as u8),
            // MMU control $EFE0–$EFE7
            0xEFE0..=0xEFE7 => self.mmu.io_read(0x10 + (addr - 0xEFE0) as u8),
            // Open-bus regions (BR-6)
            _ => self.open_bus,
        }
    }

    fn io_write(&mut self, addr: u16, val: u8) {
        let lo = (addr & 0xFF) as u8;
        match addr {
            // XT-IDE $E300–$E330 (probe writes must not crash: BR-5)
            0xE300..=0xE330 => self.xt_ide.write((addr - 0xE300) as u8, val),
            // ESP
            0xE100..=0xE102 => self.peripherals.esp_write(lo, val),
            // CH375
            0xE260..=0xE261 => self.peripherals.ch375_write(lo, val),
            // Multi-I/O
            0xE3FE..=0xE3FF => self.peripherals.multiio_write((addr - 0xE3FE) as u8, val),
            // ACIA
            0xEF84..=0xEF87 => self.acia.write((addr - 0xEF84) as u8, val),
            // RTC
            0xEF90..=0xEF9F => self.rtc.write((addr - 0xEF90) as u8, val),
            // MMU edit window
            0xEFD0..=0xEFDF => self.mmu.io_write((addr - 0xEFD0) as u8, val),
            // MMU control
            0xEFE0..=0xEFE7 => self.mmu.io_write(0x10 + (addr - 0xEFE0) as u8, val),
            // Everything else: silently discard
            _ => {}
        }
    }

    pub fn acia_mut(&mut self) -> &mut Acia {
        &mut self.acia
    }

    pub fn mmu(&self) -> &Mmu {
        &self.mmu
    }
}
