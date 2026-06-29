// Gate test for WI-M6: RTC model, config-file boot, configuration hardening.
//
// Covers REQ-M6 items 1–6, TS-6, TS-9, BR-6, BR-7, OQ-R0.5.
// These tests exercise the hardware model directly and do not require rom.hex
// or a disk image.

use emulator::bus::Bus;
use emulator::config::{Config, RomBank, RtcPolicy};
use emulator::rom::Rom;

fn blank_bus(cfg: Config) -> Bus {
    let rom = Rom::blank(cfg.rom_bank.clone());
    Bus::new(&cfg, rom, None)
}

// REQ-M6 item 1 / TS-6: RTC host policy returns a plausible year (2020–2040).
#[test]
fn rtc_host_year_plausible() {
    let mut cfg = Config::default();
    cfg.rtc_policy = RtcPolicy::Host;
    let mut bus = blank_bus(cfg);
    // Reg $0C = tens digit of 2-digit year, reg $0B = ones digit.
    let yr_tens = bus.read(0xEF9C) as u16;
    let yr_ones = bus.read(0xEF9B) as u16;
    let two_digit_year = yr_tens * 10 + yr_ones;
    assert!(
        (20..=40).contains(&two_digit_year),
        "expected 2-digit year 20–40, got {two_digit_year}"
    );
}

// REQ-M6 item 2 / TS-6: writing the documented control sequence produces no
// fault, and the clock still responds after the sequence.
#[test]
fn rtc_control_sequence_no_fault() {
    let cfg = Config::default(); // host policy
    let mut bus = blank_bus(cfg);
    // Write documented control sequence ($02/$00/$00/$01/$05/$04) to RTC
    // offsets $0A–$0F (bus addresses $EF9A–$EF9F).  The last byte ($04) clears
    // STOP (bit 3 = 0), so the clock is not frozen afterward.
    let seq = [0x02u8, 0x00, 0x00, 0x01, 0x05, 0x04];
    for (i, &val) in seq.iter().enumerate() {
        bus.write(0xEF9A + i as u16, val);
    }
    // Clock must respond without panic; reading any time register is the proof.
    let _ = bus.read(0xEF90); // 1s-digit of seconds
}

// REQ-M6 item 1 / TS-6: fixed policy returns the configured epoch date.
// Epoch 1736937000 = 2025-01-15 10:30:00 UTC.
#[test]
fn rtc_fixed_matches_epoch() {
    // 2025-01-15 10:30:00 UTC: sec=0, min=30, hour=10, weekday=4(Wed),
    // day=15, month=1, year=2025 → BCD 2-digit year 25.
    let mut cfg = Config::default();
    cfg.rtc_policy = RtcPolicy::Fixed;
    cfg.rtc_epoch = 1_736_937_000;
    let mut bus = blank_bus(cfg);

    assert_eq!(bus.read(0xEF90), 0, "sec ones");
    assert_eq!(bus.read(0xEF91), 0, "sec tens");
    assert_eq!(bus.read(0xEF92), 0, "min ones");
    assert_eq!(bus.read(0xEF93), 3, "min tens");
    assert_eq!(bus.read(0xEF94), 0, "hour ones");
    assert_eq!(bus.read(0xEF95), 1, "hour tens");
    assert_eq!(bus.read(0xEF96), 4, "weekday (Wed=4)");
    assert_eq!(bus.read(0xEF97), 5, "day ones (15)");
    assert_eq!(bus.read(0xEF98), 1, "day tens (15)");
    assert_eq!(bus.read(0xEF99), 1, "month ones (Jan)");
    assert_eq!(bus.read(0xEF9A), 0, "month tens (Jan)");
    assert_eq!(bus.read(0xEF9B), 5, "year ones (25)");
    assert_eq!(bus.read(0xEF9C), 2, "year tens (25)");
}

// REQ-M6 item 3 / BR-7: CH375 reads return open-bus and do not crash.
#[test]
fn ch375_returns_open_bus_no_crash() {
    let mut cfg = Config::default();
    cfg.open_bus.value = 0xAB;
    let mut bus = blank_bus(cfg);
    assert_eq!(bus.read(0xE260), 0xAB, "CH375 data port");
    assert_eq!(bus.read(0xE261), 0xAB, "CH375 command/status port");
}

// REQ-M6 item 4 / BR-7: Multi-I/O $AA command → $55 response (keyboard self-test).
#[test]
fn multiio_selftest_aa_55() {
    let cfg = Config::default();
    let mut bus = blank_bus(cfg);
    bus.write(0xE3FE, 0xAA);
    assert_eq!(bus.read(0xE3FE), 0x55, "Multi-I/O $AA → $55");
}

// REQ-M6 item 5 / BR-6 / OQ-R0.5: unmapped $EFA0 returns configured open-bus value.
#[test]
fn open_bus_at_efa0() {
    let mut cfg = Config::default();
    cfg.open_bus.value = 0xEA;
    let mut bus = blank_bus(cfg);
    assert_eq!(bus.read(0xEFA0), 0xEA);
    // Verify the full open-bus region $EFA0–$EFCF also returns open-bus.
    for addr in 0xEFA0u16..=0xEFCFu16 {
        assert_eq!(bus.read(addr), 0xEA, "open-bus at ${addr:04X}");
    }
}

// REQ-M6 item 6 / TS-9 / OQ-R0.4/R0.5: config loaded from TOML string applies
// non-default rom_bank, open_bus, and rtc_policy; emulator Bus starts without panic.
#[test]
fn config_from_toml_applies_settings() {
    let toml = "\
rom_bank = \"video\"\n\
rtc_policy = \"host\"\n\
[open_bus]\n\
value = 234\n"; // 234 = 0xEA
    let cfg = Config::from_toml_str(toml);
    assert!(matches!(cfg.rom_bank, RomBank::Video), "rom_bank must be video");
    assert!(matches!(cfg.rtc_policy, RtcPolicy::Host), "rtc_policy must be host");
    assert_eq!(cfg.open_bus.value, 0xEA, "open_bus must be 0xEA");
    // Verify the Bus initialises without panic using those settings.
    let _bus = blank_bus(cfg);
}
