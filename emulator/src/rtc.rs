use crate::config::RtcPolicy;

/// RTC-72421/72423 at CPU $EF90–$EF9F.
///
/// 16 registers, each storing only the low 4 bits of the written value.
/// Offsets $0D–$0F are control registers.
///
/// Clock policies (rtc_policy config):
///   host  — mirror the host system clock
///   fixed — return a fixed, configured epoch
///   epoch — advance from a configured start time on each read
pub struct Rtc {
    regs: [u8; 16],
    policy: RtcPolicy,
    stopped: bool,
}

impl Rtc {
    pub fn new(cfg: &crate::config::Config) -> Self {
        Rtc {
            regs: [0; 16],
            policy: cfg.rtc_policy.clone(),
            stopped: false,
        }
    }

    pub fn read(&mut self, offset: u8) -> u8 {
        let idx = (offset & 0x0F) as usize;
        if !self.stopped {
            self.update_from_policy();
        }
        self.regs[idx] & 0x0F
    }

    pub fn write(&mut self, offset: u8, val: u8) {
        let idx = (offset & 0x0F) as usize;
        self.regs[idx] = val & 0x0F;
        // Control register $0F: bit 3 = STOP, bit 2 = RESET (stub handling)
        if idx == 0x0F {
            self.stopped = (val & 0x08) != 0;
        }
    }

    fn update_from_policy(&mut self) {
        match self.policy {
            RtcPolicy::Host => {
                // Stub: populate BCD year/month/day from host time.
                // Full implementation in WI-M6.
            }
            RtcPolicy::Fixed | RtcPolicy::Epoch => {
                // Stub: use fixed 2025-01-01 00:00:00 for now.
            }
        }
    }
}
