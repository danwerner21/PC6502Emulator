use crate::config::RomBank;

/// ROM image: two 4 KiB banks (Base and VIDEO), selected via config.
pub struct Rom {
    base: [u8; 4096],
    video: [u8; 4096],
    bank: RomBank,
}

impl Rom {
    /// Load from an Intel HEX file. The hex encodes 8 KiB; split into two 4 KiB banks.
    pub fn load_hex(path: &str, bank: RomBank) -> std::io::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        let mut buf = [0u8; 8192];
        parse_intel_hex(&text, &mut buf);
        let mut base = [0u8; 4096];
        let mut video = [0u8; 4096];
        base.copy_from_slice(&buf[0..4096]);
        video.copy_from_slice(&buf[4096..8192]);
        Ok(Rom { base, video, bank })
    }

    /// Create a blank ROM (all $FF) for testing without a real hex file.
    pub fn blank(bank: RomBank) -> Self {
        Rom {
            base: [0xFF; 4096],
            video: [0xFF; 4096],
            bank,
        }
    }

    /// Read a byte from the active bank. `offset` is 0–4095 relative to $F000.
    pub fn read(&self, offset: u16) -> u8 {
        let idx = (offset & 0x0FFF) as usize;
        match self.bank {
            RomBank::Base => self.base[idx],
            RomBank::Video => self.video[idx],
        }
    }
}

/// Parse Intel HEX records into `buf`. Ignores unknown record types.
fn parse_intel_hex(text: &str, buf: &mut [u8]) {
    for line in text.lines() {
        let line = line.trim();
        if !line.starts_with(':') || line.len() < 11 {
            continue;
        }
        let bytes: Vec<u8> = (1..line.len())
            .step_by(2)
            .filter_map(|i| u8::from_str_radix(&line[i..i + 2], 16).ok())
            .collect();
        if bytes.len() < 5 {
            continue;
        }
        let byte_count = bytes[0] as usize;
        let address = (bytes[1] as usize) << 8 | (bytes[2] as usize);
        let record_type = bytes[3];
        if record_type == 0x00 {
            // Data record
            for (i, &b) in bytes[4..4 + byte_count].iter().enumerate() {
                let dest = address.wrapping_add(i);
                if dest < buf.len() {
                    buf[dest] = b;
                }
            }
        }
    }
}
