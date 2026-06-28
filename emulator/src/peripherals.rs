/// Safe no-op stubs for absent optional peripherals.
///
/// CH375 USB host at $E260–$E261
/// Dual ESP Wi-Fi at $E100–$E102
/// Multi-I/O keyboard at $E3FE–$E3FF
///
/// All reads return open_bus. All writes are silently discarded.
/// Multi-I/O keyboard self-test: $AA command → $55 response (BR-7 / REQ-M6).
pub struct Peripherals {
    open_bus: u8,
    /// Keyboard self-test state: true when $AA command has been issued.
    kbd_selftest_pending: bool,
}

impl Peripherals {
    pub fn new(open_bus: u8) -> Self {
        Peripherals { open_bus, kbd_selftest_pending: false }
    }

    // --- CH375 $E260–$E261 ---

    pub fn ch375_read(&self, _offset: u8) -> u8 {
        self.open_bus
    }

    pub fn ch375_write(&mut self, _offset: u8, _val: u8) {}

    // --- Dual ESP $E100–$E102 ---

    pub fn esp_read(&self, _offset: u8) -> u8 {
        self.open_bus
    }

    pub fn esp_write(&mut self, _offset: u8, _val: u8) {}

    // --- Multi-I/O keyboard $E3FE–$E3FF ---

    pub fn multiio_read(&mut self, offset: u8) -> u8 {
        if offset == 0 && self.kbd_selftest_pending {
            self.kbd_selftest_pending = false;
            return 0x55;
        }
        self.open_bus
    }

    pub fn multiio_write(&mut self, offset: u8, val: u8) {
        if offset == 0 && val == 0xAA {
            self.kbd_selftest_pending = true;
        }
    }
}
