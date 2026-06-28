/// 6502 processor status register bit positions.
pub const N: u8 = 0b1000_0000; // Negative
pub const V: u8 = 0b0100_0000; // Overflow
pub const U: u8 = 0b0010_0000; // Unused (always 1)
pub const B: u8 = 0b0001_0000; // Break
pub const D: u8 = 0b0000_1000; // Decimal
pub const I: u8 = 0b0000_0100; // Interrupt disable
pub const Z: u8 = 0b0000_0010; // Zero
pub const C: u8 = 0b0000_0001; // Carry

/// Processor status register helpers.
#[derive(Debug, Clone, Copy, Default)]
pub struct Flags(pub u8);

impl Flags {
    pub fn get(&self, flag: u8) -> bool {
        self.0 & flag != 0
    }

    pub fn set(&mut self, flag: u8, value: bool) {
        if value {
            self.0 |= flag;
        } else {
            self.0 &= !flag;
        }
    }

    pub fn set_nz(&mut self, result: u8) {
        self.set(N, result & 0x80 != 0);
        self.set(Z, result == 0);
    }
}
