use crate::disk::DiskImage;

/// XT-IDE controller at CPU $E300–$E30E.
///
/// Register map (offsets from $E300):
///   $00 — Data (16-bit port, accessed as two 8-bit reads/writes)
///   $01 — Error / Features
///   $02 — Sector Count
///   $03 — LBA 0 (bits 7:0)
///   $04 — LBA 1 (bits 15:8)
///   $05 — LBA 2 (bits 23:16)
///   $06 — LBA 3 / Drive select (bits 27:24 in low nibble)
///   $07 — Status (read) / Command (write)
///
/// Supported commands (WI-M1/M3 subset):
///   $20 — READ SECTORS
///   $EF — SET FEATURES
///   $EC — IDENTIFY
pub struct XtIde {
    status: u8,
    error: u8,
    features: u8,
    sector_count: u8,
    lba: [u8; 4],
    current_command: u8,
    transfer_buf: [u8; 512],
    buf_pos: usize,
    drq: bool,
    disk: Option<DiskImage>,
    open_bus: u8,
}

// Status register bits
const BSY: u8 = 0x80;
const DRDY: u8 = 0x40;
const DRQ: u8 = 0x08;
const ERR: u8 = 0x01;

impl XtIde {
    pub fn new(disk: Option<DiskImage>, open_bus: u8) -> Self {
        XtIde {
            status: DRDY,
            error: 0,
            features: 0,
            sector_count: 0,
            lba: [0; 4],
            current_command: 0,
            transfer_buf: [0; 512],
            buf_pos: 0,
            drq: false,
            disk,
            open_bus,
        }
    }

    /// Read a register. `offset` is relative to $E300.
    pub fn read(&mut self, offset: u8) -> u8 {
        match offset {
            0x00 => {
                // Data port — return next byte from transfer buffer
                if self.drq && self.buf_pos < 512 {
                    let b = self.transfer_buf[self.buf_pos];
                    self.buf_pos += 1;
                    if self.buf_pos >= 512 {
                        self.drq = false;
                        self.status = DRDY;
                    } else {
                        self.status = DRDY | DRQ;
                    }
                    b
                } else {
                    0
                }
            }
            0x01 => self.error,
            0x02 => self.sector_count,
            0x03 => self.lba[0],
            0x04 => self.lba[1],
            0x05 => self.lba[2],
            0x06 => self.lba[3],
            0x07 => {
                if self.drq {
                    DRDY | DRQ
                } else {
                    self.status
                }
            }
            _ => self.open_bus,
        }
    }

    /// Write a register. `offset` is relative to $E300.
    pub fn write(&mut self, offset: u8, val: u8) {
        // Probe writes to $E300–$E330 must not crash or corrupt disk state (BR-5).
        // Unknown offsets are silently discarded.
        match offset {
            0x00 => {} // Data write — stub for WI-M5
            0x01 => self.features = val,
            0x02 => self.sector_count = val,
            0x03 => self.lba[0] = val,
            0x04 => self.lba[1] = val,
            0x05 => self.lba[2] = val,
            0x06 => self.lba[3] = val,
            0x07 => self.execute_command(val),
            _ => {} // Probe tolerance: ignore writes to unknown offsets
        }
    }

    fn execute_command(&mut self, cmd: u8) {
        self.current_command = cmd;
        match cmd {
            0x20 => {
                // READ SECTORS
                let lba = self.lba_addr();
                if let Some(ref disk) = self.disk {
                    self.transfer_buf = disk.read_sector(lba);
                } else {
                    self.transfer_buf = [0; 512];
                }
                self.buf_pos = 0;
                self.drq = true;
                self.status = DRDY | DRQ;
                self.error = 0;
            }
            0xEF => {
                // SET FEATURES — BSY clears; DRQ and ERR absent
                self.status = DRDY;
                self.error = 0;
                self.drq = false;
            }
            0xEC => {
                // IDENTIFY — minimal 512-byte response (full block in WI-M5)
                self.transfer_buf = [0; 512];
                // Set model string at words 27–46 (bytes 54–93)
                let model = b"PC6502 DISK                             ";
                self.transfer_buf[54..54 + model.len()].copy_from_slice(model);
                self.buf_pos = 0;
                self.drq = true;
                self.status = DRDY | DRQ;
                self.error = 0;
            }
            _ => {
                // Unknown command
                self.status = DRDY | ERR;
                self.error = 0x04;
            }
        }
    }

    fn lba_addr(&self) -> u32 {
        (self.lba[0] as u32)
            | ((self.lba[1] as u32) << 8)
            | ((self.lba[2] as u32) << 16)
            | ((self.lba[3] as u32 & 0x0F) << 24)
    }
}
