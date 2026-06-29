use std::path::Path;

const SECTOR_SIZE: usize = 512;

/// Raw flat disk image for XT-IDE sector transfers.
pub struct DiskImage {
    data: Vec<u8>,
    path: Option<String>,
}

impl DiskImage {
    /// Load a flat binary disk image from a file.
    pub fn load(path: &str) -> std::io::Result<Self> {
        let data = std::fs::read(path)?;
        Ok(DiskImage { data, path: Some(path.to_string()) })
    }

    /// Create an empty (zero-filled) image of the given sector count.
    pub fn blank(sectors: u32) -> Self {
        DiskImage {
            data: vec![0u8; sectors as usize * SECTOR_SIZE],
            path: None,
        }
    }

    /// Read one 512-byte sector by LBA index.
    pub fn read_sector(&self, lba: u32) -> [u8; 512] {
        let offset = lba as usize * SECTOR_SIZE;
        let mut sector = [0u8; 512];
        if offset + SECTOR_SIZE <= self.data.len() {
            sector.copy_from_slice(&self.data[offset..offset + SECTOR_SIZE]);
        }
        sector
    }

    /// Write one 512-byte sector to the in-memory image. Flush to persist.
    pub fn write_sector(&mut self, lba: u32, data: &[u8; 512]) {
        let offset = lba as usize * SECTOR_SIZE;
        if offset + SECTOR_SIZE > self.data.len() {
            self.data.resize(offset + SECTOR_SIZE, 0);
        }
        self.data[offset..offset + SECTOR_SIZE].copy_from_slice(data);
    }

    /// Flush written sectors to the backing file if one is set.
    pub fn flush(&self) -> std::io::Result<()> {
        if let Some(ref p) = self.path {
            std::fs::write(Path::new(p), &self.data)?;
        }
        Ok(())
    }

    pub fn sector_count(&self) -> u32 {
        (self.data.len() / SECTOR_SIZE) as u32
    }
}
