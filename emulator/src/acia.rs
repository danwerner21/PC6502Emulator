use crate::config::{AciaVariant, Config};

/// 6551-compatible ACIA at CPU $EF84–$EF87.
///
/// Registers:
///   $EF84 — Data register (TX write / RX read)
///   $EF85 — Status register (read) / Programmed reset (write $00)
///   $EF86 — Command register
///   $EF87 — Control register
///
/// TX path: byte written to $EF84 is emitted to stdout. TDRE (bit 4 of status)
/// is always set — the emulator never back-pressures the transmitter.
/// RX path: RDRF (bit 3) is set when a byte has been injected via `inject_rx`.
pub struct Acia {
    status: u8,
    command: u8,
    control: u8,
    rx_data: u8,
    rx_ready: bool,
    _variant: AciaVariant,
    /// CTS signal; when false the TX path is blocked (OQ-R1.4).
    cts: bool,
}

/// Bit 4 of status — Transmit Data Register Empty (transmitter ready).
const TDRE: u8 = 0b0001_0000;
/// Bit 3 of status — Receive Data Register Full (byte available).
const RDRF: u8 = 0b0000_1000;

impl Acia {
    pub fn new(cfg: &Config) -> Self {
        Acia {
            status: TDRE,
            command: 0,
            control: 0,
            rx_data: 0,
            rx_ready: false,
            _variant: cfg.acia_variant.clone(),
            cts: cfg.acia_cts_default,
        }
    }

    pub fn read(&mut self, offset: u8) -> u8 {
        match offset {
            0 => {
                // RX data — clears RDRF
                let val = self.rx_data;
                self.rx_ready = false;
                self.update_status();
                val
            }
            1 => self.status,
            2 => self.command,
            3 => self.control,
            _ => 0,
        }
    }

    pub fn write(&mut self, offset: u8, val: u8) {
        match offset {
            0 => {
                // TX data
                if self.cts {
                    print!("{}", val as char);
                }
            }
            1 => {
                // Programmed reset when $00 is written
                if val == 0x00 {
                    self.programmed_reset();
                }
            }
            2 => self.command = val,
            3 => self.control = val,
            _ => {}
        }
    }

    /// Inject a byte into the RX buffer. Sets RDRF.
    pub fn inject_rx(&mut self, byte: u8) {
        self.rx_data = byte;
        self.rx_ready = true;
        self.update_status();
    }

    fn programmed_reset(&mut self) {
        self.command = 0;
        self.control = 0;
        self.rx_ready = false;
        self.status = TDRE;
    }

    fn update_status(&mut self) {
        if self.rx_ready {
            self.status |= RDRF;
        } else {
            self.status &= !RDRF;
        }
        // TDRE is always set — emulator never withholds TX-ready
        self.status |= TDRE;
    }
}
