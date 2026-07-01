use crate::disk::DiskImage;

/// XT-IDE controller at CPU $E300–$E30E.
///
/// Register map (offsets from $E300), matching the real XT-CF-LITE hardware:
///   $00 — Data LO (low byte of 16-bit data port; advances transfer buffer)
///   $01 — Data HI (high byte of 16-bit data port; advances transfer buffer)
///   $02 — Error (read) / Features (write)
///   $03 — (open bus)
///   $04 — Sector Count
///   $05 — (open bus)
///   $06 — LBA bits 7:0
///   $07 — (open bus)
///   $08 — LBA bits 15:8
///   $09 — (open bus)
///   $0A — LBA bits 23:16
///   $0B — (open bus)
///   $0C — Device / LBA bits 27:24
///   $0D — (open bus)
///   $0E — Status (read) / Command (write)
///
/// Supported commands:
///   $20 — READ SECTORS
///   $30 — WRITE SECTORS
///   $EF — SET FEATURES
///   $EC — IDENTIFY
///
/// Data access: both Data LO ($00) and Data HI ($01) advance the transfer
/// buffer position by one byte per access, enabling the firmware's interleaved
/// LO/HI read pattern (256 pairs × 2 = 512 bytes) as well as simple sequential
/// reads (512 reads from LO alone).
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
            // Data LO and Data HI both read the next byte from the transfer buffer.
            // The firmware interleaves LO/HI accesses; each advances buf_pos by one.
            0x00 | 0x01 => {
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
            0x02 => self.error,
            0x04 => self.sector_count,
            0x06 => self.lba[0],
            0x08 => self.lba[1],
            0x0A => self.lba[2],
            0x0C => self.lba[3],
            0x0E => {
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
            // Data LO and Data HI both write the next byte to the transfer buffer.
            0x00 | 0x01 => {
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
            0x02 => self.features = val,
            0x04 => self.sector_count = val,
            0x06 => self.lba[0] = val,
            0x08 => self.lba[1] = val,
            0x0A => self.lba[2] = val,
            0x0C => self.lba[3] = val,
            0x0E => self.execute_command(val),
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
                // Unknown command — set ERR, clear DRQ
                self.status = DRDY | ERR;
                self.error = 0x04;
                self.drq = false;
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
