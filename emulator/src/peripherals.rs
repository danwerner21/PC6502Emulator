/// Safe no-op stubs for absent optional peripherals.
///
/// CH375 USB host at $E260–$E261
/// Dual ESP Wi-Fi at $E100–$E102
/// Multi-I/O keyboard at $E3FE–$E3FF
///
/// ESP reads return values tuned so the DOS/65 driver's bit-test polling loops
/// all exit immediately rather than running 65K-cycle timeouts:
///
/// $E102 (status): 0x09 satisfies all six AND/branch patterns in the driver:
///   AND #$02; BEQ — bit1=0 (not busy)
///   AND #$01; BNE — bit0=1 (data available)
///   AND #$10; BEQ — bit4=0
///   AND #$08; BNE — bit3=1
///
/// $E100/$E101 (data registers): 0x01.  Driver fn 10 ($C948) sends command $03
/// and waits for a non-zero, non-$FF byte from $E100; if the response is $00 it
/// maps to $FF and fn 11 ($C95C) loops forever on `CMP #$FF; BEQ`.  Returning
/// 0x01 lets fn 10 return success and fn 11 exit cleanly.
///
/// CH375 returns 0x00 (no device present).
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
        0x00
    }

    pub fn ch375_write(&mut self, _offset: u8, _val: u8) {}

    // --- Dual ESP $E100–$E102 ---

    pub fn esp_read(&self, offset: u8) -> u8 {
        if offset == 2 { 0x09 } else { 0x01 }
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
        // Real firmware writes $AA to KBD_CMD ($E3FF, offset 1), not KBD_DAT
        // ($E3FE, offset 0) — bios_multi.asm:173-174,292 (KBD_PUTCMD's STA KBD_CMD).
        if offset == 1 && val == 0xAA {
            self.kbd_selftest_pending = true;
        }
    }
}
