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
/// Supported commands:
///   $20 — READ SECTORS
///   $30 — WRITE SECTORS
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
    write_mode: bool,
    bad_sectors: Vec<u32>,
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
            write_mode: false,
            bad_sectors: Vec::new(),
            disk,
            open_bus,
        }
    }

    /// Mark `lba` as bad: READ/WRITE SECTORS to that LBA will return ERR.
    pub fn inject_bad_sector(&mut self, lba: u32) {
        if !self.bad_sectors.contains(&lba) {
            self.bad_sectors.push(lba);
        }
    }

    /// Remove a previously-injected bad sector.
    pub fn clear_bad_sector(&mut self, lba: u32) {
        self.bad_sectors.retain(|&x| x != lba);
    }

    fn trace_enabled() -> bool {
        std::env::var("XTIDE_TRACE").map(|v| v == "1").unwrap_or(false)
    }

    /// Read a register. `offset` is relative to $E300.
    pub fn read(&mut self, offset: u8) -> u8 {
        let result = self.read_inner(offset);
        if Self::trace_enabled() {
            eprintln!("XtIde R off={:#04x} val={:#04x} drq={} pos={}", offset, result, self.drq, self.buf_pos);
        }
        result
    }

    fn read_inner(&mut self, offset: u8) -> u8 {
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
        if Self::trace_enabled() {
            eprintln!("XtIde W off={:#04x} val={:#04x}", offset, val);
        }
        // Probe writes to $E300–$E330 must not crash or corrupt disk state (BR-5).
        // Unknown offsets are silently discarded.
        match offset {
            0x00 => {
                // Data port write — accepted only during WRITE SECTORS transfer
                if self.write_mode && self.buf_pos < 512 {
                    self.transfer_buf[self.buf_pos] = val;
                    self.buf_pos += 1;
                    if self.buf_pos >= 512 {
                        // All 512 bytes received — commit to disk
                        let lba = self.lba_addr();
                        if let Some(ref mut disk) = self.disk {
                            disk.write_sector(lba, &self.transfer_buf);
                        }
                        self.write_mode = false;
                        self.drq = false;
                        self.status = DRDY;
                        self.error = 0;
                    }
                }
            }
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
                if self.bad_sectors.contains(&lba) {
                    self.status = DRDY | ERR;
                    self.error = 0x04; // ABRT
                    self.drq = false;
                    self.write_mode = false;
                    return;
                }
                if let Some(ref disk) = self.disk {
                    self.transfer_buf = disk.read_sector(lba);
                } else {
                    self.transfer_buf = [0; 512];
                }
                self.buf_pos = 0;
                self.drq = true;
                self.write_mode = false;
                self.status = DRDY | DRQ;
                self.error = 0;
            }
            0x30 => {
                // WRITE SECTORS — set DRQ to request 512-byte data transfer from CPU
                let lba = self.lba_addr();
                if self.bad_sectors.contains(&lba) {
                    self.status = DRDY | ERR;
                    self.error = 0x04; // ABRT
                    self.drq = false;
                    self.write_mode = false;
                    return;
                }
                self.transfer_buf = [0; 512];
                self.buf_pos = 0;
                self.drq = true;
                self.write_mode = true;
                self.status = DRDY | DRQ;
                self.error = 0;
            }
            0xEF => {
                // SET FEATURES — BSY clears; DRQ and ERR absent
                self.status = DRDY;
                self.error = 0;
                self.drq = false;
                self.write_mode = false;
            }
            0xEC => {
                // IDENTIFY — 512-byte response per ATA spec
                self.transfer_buf = [0; 512];
                // Word 0 (bytes 0-1): general config — fixed disk
                self.transfer_buf[0] = 0x5A;
                self.transfer_buf[1] = 0x04;
                // Word 49 (bytes 98-99): capabilities — LBA supported (bit 9 of word = bit 1 of byte 99)
                self.transfer_buf[99] = 0x02;
                // Words 60-61 (bytes 120-123): total LBA sectors (little-endian 32-bit)
                let total = self.disk.as_ref().map(|d| d.sector_count()).unwrap_or(0);
                self.transfer_buf[120] = (total & 0xFF) as u8;
                self.transfer_buf[121] = ((total >> 8) & 0xFF) as u8;
                self.transfer_buf[122] = ((total >> 16) & 0xFF) as u8;
                self.transfer_buf[123] = ((total >> 24) & 0xFF) as u8;
                // Words 27-46 (bytes 54-93): model string (40 bytes, space-padded)
                let model = b"PC6502 DISK                             ";
                self.transfer_buf[54..54 + model.len()].copy_from_slice(model);
                self.buf_pos = 0;
                self.drq = true;
                self.write_mode = false;
                self.status = DRDY | DRQ;
                self.error = 0;
            }
            _ => {
                // Unknown command
                self.status = DRDY | ERR;
                self.error = 0x04;
                self.write_mode = false;
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
