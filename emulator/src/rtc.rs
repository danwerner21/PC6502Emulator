use crate::config::RtcPolicy;

/// RTC-72421/72423 at CPU $EF90–$EF9F.
///
/// 16 registers, each storing only the low 4 bits of the written value.
/// Offsets $0D–$0F are control registers.
/// Register $0F bit 1: STOP (1 = freeze counter). Bit 3 is TEST, not STOP.
///
/// Clock policies (rtc_policy config):
///   host  — mirror the host system clock (Linux wall clock via SystemTime)
///   fixed — return a configured fixed epoch (rtc_epoch Unix timestamp)
///   epoch — advance from a configured start time (uses same rtc_epoch)
pub struct Rtc {
    regs: [u8; 16],
    policy: RtcPolicy,
    epoch: u64,
    stopped: bool,
}

impl Rtc {
    pub fn new(cfg: &crate::config::Config) -> Self {
        Rtc {
            regs: [0; 16],
            policy: cfg.rtc_policy.clone(),
            epoch: cfg.rtc_epoch,
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
        if idx == 0x0F {
            self.stopped = (val & 0x02) != 0;
        }
    }

    fn update_from_policy(&mut self) {
        match self.policy {
            RtcPolicy::Host => {
                let secs = host_unix_secs();
                self.populate_from_unix(secs);
            }
            RtcPolicy::Fixed | RtcPolicy::Epoch => {
                self.populate_from_unix(self.epoch);
            }
        }
    }

    fn populate_from_unix(&mut self, secs: u64) {
        let (year, month, day, weekday, hour, min, sec) = unix_to_calendar(secs);
        let yr2 = (year % 100) as u8;
        self.regs[0x00] = sec % 10;
        self.regs[0x01] = sec / 10;
        self.regs[0x02] = min % 10;
        self.regs[0x03] = min / 10;
        self.regs[0x04] = hour % 10;
        self.regs[0x05] = hour / 10;
        self.regs[0x06] = day % 10;
        self.regs[0x07] = day / 10;
        self.regs[0x08] = month % 10;
        self.regs[0x09] = month / 10;
        self.regs[0x0A] = yr2 % 10;
        self.regs[0x0B] = yr2 / 10;
        self.regs[0x0C] = weekday;
        // $0D–$0F are control registers; leave them untouched by clock updates.
    }
}

/// Returns current Unix time in seconds, or 0 on failure.
fn host_unix_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Convert Unix timestamp to (year, month, day, weekday, hour, min, sec).
///
/// weekday uses 1=Sun..7=Sat (TC72421 convention).
/// month is 1-indexed. year is the full four-digit year.
fn unix_to_calendar(secs: u64) -> (u16, u8, u8, u8, u8, u8, u8) {
    let sec = (secs % 60) as u8;
    let secs_in_day = secs % 86400;
    let min = ((secs_in_day / 60) % 60) as u8;
    let hour = (secs_in_day / 3600) as u8;
    let days = (secs / 86400) as u32;

    // Jan 1 1970 was a Thursday; in 1=Sun scheme Thursday=5, offset=4.
    let weekday = ((days + 4) % 7 + 1) as u8;

    let mut year = 1970u16;
    let mut remaining = days;
    loop {
        let diy = if is_leap(year) { 366u32 } else { 365 };
        if remaining < diy {
            break;
        }
        remaining -= diy;
        year += 1;
    }

    let month_lens: [u32; 12] = [
        31,
        if is_leap(year) { 29 } else { 28 },
        31, 30, 31, 30, 31, 31, 30, 31, 30, 31,
    ];
    let mut month = 1u8;
    for &ml in month_lens.iter() {
        if remaining < ml {
            break;
        }
        remaining -= ml;
        month += 1;
    }
    let day = (remaining + 1) as u8;

    (year, month, day, weekday, hour, min, sec)
}

fn is_leap(year: u16) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}
