use crate::config::{AciaVariant, Config};
use std::collections::VecDeque;

/// 6551-compatible ACIA at CPU $EF84–$EF87.
///
/// Registers:
///   $EF84 — Data register (TX write / RX read)
///   $EF85 — Status register (read) / Programmed reset (write $00)
///   $EF86 — Command register
///   $EF87 — Control register
///
/// TX path: byte written to $EF84 is buffered in `tx_buf`; the binary
/// drains this to stdout.  TDRE (bit 4 of status) is always set.
/// RX path: RDRF (bit 3) is set when a byte is available in `rx_queue`.
pub struct Acia {
    status: u8,
    command: u8,
    control: u8,
    rx_queue: VecDeque<u8>,
    _variant: AciaVariant,
    /// CTS signal; when false the TX path is blocked (OQ-R1.4).
    cts: bool,
    /// Accumulates all bytes sent to TDR, in order.
    pub tx_buf: Vec<u8>,
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
            rx_queue: VecDeque::new(),
            _variant: cfg.acia_variant.clone(),
            cts: cfg.acia_cts_default,
            tx_buf: Vec::new(),
        }
    }

    pub fn read(&mut self, offset: u8) -> u8 {
        match offset {
            0 => {
                // RX data — pop next byte from queue, update RDRF
                let val = self.rx_queue.pop_front().unwrap_or(0);
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
                    self.tx_buf.push(val);
                }
            }
            1 => {
                // Programmed reset: classic-compatible (C10/C11)
                // Clears command bits 4:0, preserves parity bits 7:5; control unchanged.
                self.command &= 0b1110_0000;
                self.rx_queue.clear();
                self.status = TDRE;
                let _ = val; // written value is ignored
            }
            2 => self.command = val,
            3 => self.control = val,
            _ => {}
        }
    }

    /// Inject bytes into the receive queue.  Sets RDRF when at least one byte is present.
    pub fn inject_rx(&mut self, byte: u8) {
        self.rx_queue.push_back(byte);
        self.update_status();
    }

    /// Inject a slice of bytes into the receive queue.
    pub fn inject_rx_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.rx_queue.push_back(b);
        }
        self.update_status();
    }

    /// Take all accumulated TX bytes, leaving the buffer empty.
    pub fn drain_output(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.tx_buf)
    }

    /// Borrow the accumulated TX bytes without draining.
    pub fn output(&self) -> &[u8] {
        &self.tx_buf
    }

    fn update_status(&mut self) {
        if self.rx_queue.is_empty() {
            self.status &= !RDRF;
        } else {
            self.status |= RDRF;
        }
        // TDRE is always set — emulator never withholds TX-ready
        self.status |= TDRE;
    }
}
