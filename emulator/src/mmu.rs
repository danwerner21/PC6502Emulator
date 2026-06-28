use crate::config::{Config, MmuPowerOnFill};

/// 64-task MMU with 1,024-byte map store.
///
/// Map store: 64 tasks × 16 bytes per task = 1,024 bytes.
/// Each byte maps one 4 KiB logical page to a physical page number.
/// 16 pages per task covers the full 64 KiB CPU address space.
///
/// Edit window:    CPU $EFD0–$EFDF (16 bytes) — read/write the setup task's map
/// Control regs:  CPU $EFE0–$EFE7
///   $EFE0 — Active task register (write → active_task = val & 0x3F)
///   $EFE1 — Setup task register (write → setup_task = val & 0x3F; selects edit window task)
///   $EFE2 — Enable register (write $01 → MMU enabled)
///   $EFE3 — (reserved)
///   $EFE4 — Status: bits[5:0] = active task, bit[7] = enabled
///   $EFE5–$EFE7 — (reserved)
pub struct Mmu {
    map: [u8; 1024],
    active_task: u8,
    setup_task: u8,
    enabled: bool,
    open_bus: u8,
}

impl Mmu {
    pub fn new(cfg: &Config) -> Self {
        let fill_value: u8 = match &cfg.mmu_power_on_fill {
            MmuPowerOnFill::Zero => 0x00,
            MmuPowerOnFill::Fixed(v) => *v,
            MmuPowerOnFill::Random => {
                // Use a simple deterministic pseudo-random fill for reproducibility;
                // true randomness here would make tests non-deterministic.
                0xA5
            }
        };
        Mmu {
            map: [fill_value; 1024],
            active_task: 0,
            setup_task: 0,
            enabled: false,
            open_bus: cfg.open_bus.value,
        }
    }

    /// Translate a logical page number (0–15) for the given task to a physical page.
    pub fn translate(&self, task: u8, logical_page: u8) -> u8 {
        let idx = (task as usize) * 16 + (logical_page as usize & 0x0F);
        self.map[idx]
    }

    /// Translate a CPU 16-bit address to a 20-bit physical address using the
    /// active task's map. Returns None when MMU is disabled (identity mapping).
    pub fn translate_addr(&self, addr: u16) -> Option<u32> {
        if !self.enabled {
            return None;
        }
        let logical_page = (addr >> 12) as u8;
        let physical_page = self.translate(self.active_task, logical_page) as u32;
        let offset = (addr & 0x0FFF) as u32;
        Some((physical_page << 12) | offset)
    }

    /// Read from edit window ($EFD0–$EFDF) or control registers ($EFE0–$EFE7).
    pub fn io_read(&self, offset: u8) -> u8 {
        match offset {
            0x00..=0x0F => {
                // Edit window — setup task's map entries (set by $EFE1)
                let idx = (self.setup_task as usize) * 16 + (offset as usize);
                self.map[idx]
            }
            0x10 => {
                // $EFE0 — task mask (write-only by convention; read returns open-bus)
                self.open_bus
            }
            0x11 => self.open_bus,
            0x12 => {
                // $EFE2 — enable register (write-only; read returns open-bus)
                self.open_bus
            }
            0x13 => self.open_bus,
            0x14 => {
                // $EFE4 — status
                let enabled_bit = if self.enabled { 0x80 } else { 0x00 };
                enabled_bit | (self.active_task & 0x3F)
            }
            _ => self.open_bus,
        }
    }

    /// Write to edit window ($EFD0–$EFDF) or control registers ($EFE0–$EFE7).
    pub fn io_write(&mut self, offset: u8, val: u8) {
        match offset {
            0x00..=0x0F => {
                // Edit window — setup task's map entries (set by $EFE1)
                let idx = (self.setup_task as usize) * 16 + (offset as usize);
                self.map[idx] = val;
            }
            0x10 => {
                // $EFE0 — active task register; task mask clamps to 6 bits
                self.active_task = val & 0x3F;
            }
            0x11 => {
                // $EFE1 — setup task register; selects which task's map the edit window exposes
                self.setup_task = val & 0x3F;
            }
            0x12 => {
                // $EFE2 — enable: bit 0 enables MMU translation
                self.enabled = val & 0x01 != 0;
            }
            _ => {}
        }
    }

    pub fn active_task(&self) -> u8 {
        self.active_task
    }

    pub fn setup_task(&self) -> u8 {
        self.setup_task
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}
