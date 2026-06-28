pub mod flags;
pub mod opcodes;

use flags::Flags;

/// 6502 CPU state.
#[derive(Debug, Default)]
pub struct Cpu {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,
    pub pc: u16,
    pub p: Flags,
}

impl Cpu {
    pub fn new() -> Self {
        Cpu {
            sp: 0xFF,
            ..Default::default()
        }
    }

    /// Execute one instruction via the provided bus read/write callbacks.
    /// Returns the number of cycles consumed.
    /// Stub: full implementation in WI-M1.
    pub fn step<R, W>(&mut self, _read: R, _write: W) -> u32
    where
        R: FnMut(u16) -> u8,
        W: FnMut(u16, u8),
    {
        0
    }

    /// Drive the RESET sequence: load PC from $FFFC/$FFFD, set I flag.
    pub fn reset<R>(&mut self, mut read: R)
    where
        R: FnMut(u16) -> u8,
    {
        let lo = read(0xFFFC) as u16;
        let hi = read(0xFFFD) as u16;
        self.pc = (hi << 8) | lo;
        self.p.set(flags::I, true);
        self.sp = 0xFF;
    }
}
