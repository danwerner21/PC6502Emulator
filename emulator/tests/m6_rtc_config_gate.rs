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
    // Reg $0B = tens digit of 2-digit year, reg $0A = ones digit.
    let yr_tens = bus.read(0xEF9B) as u16;
    let yr_ones = bus.read(0xEF9A) as u16;
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
    // STOP (bit 1 = 0), so the clock is not frozen afterward.
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
    assert_eq!(bus.read(0xEF96), 5, "day ones (15)");
    assert_eq!(bus.read(0xEF97), 1, "day tens (15)");
    assert_eq!(bus.read(0xEF98), 1, "month ones (Jan)");
    assert_eq!(bus.read(0xEF99), 0, "month tens (Jan)");
    assert_eq!(bus.read(0xEF9A), 5, "year ones (25)");
    assert_eq!(bus.read(0xEF9B), 2, "year tens (25)");
    assert_eq!(bus.read(0xEF9C), 4, "weekday (Wed=4)");
}

// REQ-M6 item 3 / BR-7: CH375 reads do not crash; return 0x00 (no USB device present).
#[test]
fn ch375_returns_open_bus_no_crash() {
    let cfg = Config::default();
    let mut bus = blank_bus(cfg);
    assert_eq!(bus.read(0xE260), 0x00, "CH375 data port (no device)");
    assert_eq!(bus.read(0xE261), 0x00, "CH375 command/status port (no device)");
}

// REQ-M6 item 4 / BR-7: Multi-I/O $AA command → $55 response (keyboard self-test).
// Real firmware (bios_multi.asm:173-174,292 KBD_PUTCMD) writes $AA to KBD_CMD
// ($E3FF, offset 1) and (bios_multi.asm:176,361 KBD_GETDATA) reads the response
// from KBD_DAT ($E3FE, offset 0) — the two registers are distinct.
#[test]
fn multiio_selftest_aa_55() {
    let cfg = Config::default();
    let mut bus = blank_bus(cfg);
    bus.write(0xE3FF, 0xAA);
    assert_eq!(bus.read(0xE3FE), 0x55, "Multi-I/O $AA → $55");
}

// Regression guard for the offset bug fixed alongside this test: a write to
// KBD_DAT ($E3FE), the address the emulator incorrectly checked before the
// fix, must NOT arm the self-test response.
#[test]
fn multiio_selftest_wrong_port_is_not_armed() {
    let cfg = Config::default();
    let open_bus = cfg.open_bus.value;
    let mut bus = blank_bus(cfg);
    bus.write(0xE3FE, 0xAA);
    assert_eq!(
        bus.read(0xE3FE),
        open_bus,
        "Multi-I/O: $AA written to the data port ($E3FE) must not arm the self-test"
    );
}

// Regression guard for mc-hpg: with the default open_bus ($EA, bit1=1), real
// firmware's KBD_PUTCMD busy-wait (bios_multi.asm:269-283, AND #$02/BEQ on
// KBD_ST bit1) polled raw open_bus and always saw "busy", timing out before
// ever writing $AA — producing "KBD: VT82C42 WRITE TIMEOUT." on every real
// boot even though the mc-zrr offset fix was correct. $E3FF bit1 must read
// clear regardless of open_bus's own bit1.
#[test]
fn multiio_status_never_reports_busy_even_when_open_bus_bit1_is_set() {
    let mut cfg = Config::default();
    cfg.open_bus.value = 0xEA; // bit1 = 1 in the raw open-bus byte
    let mut bus = blank_bus(cfg);
    assert_eq!(
        bus.read(0xE3FF) & 0x02,
        0x00,
        "Multi-I/O $E3FF bit1 (busy) must read clear so KBD_PUTCMD's busy-wait succeeds"
    );
}

// Regression guard for mc-hpg: $E3FF bit0 (data-pending) must reflect queued
// self-test response state, not float on raw open_bus — otherwise KBD_GETDATA's
// poll (bios_multi.asm:326-338, AND #$01/BNE on bit0) times out even after the
// $AA command was correctly armed.
#[test]
fn multiio_status_data_pending_tracks_selftest_state() {
    let cfg = Config::default();
    let mut bus = blank_bus(cfg);
    assert_eq!(bus.read(0xE3FF) & 0x01, 0x00, "no data pending before $AA");
    bus.write(0xE3FF, 0xAA);
    assert_eq!(bus.read(0xE3FF) & 0x01, 0x01, "data pending after $AA armed");
    bus.read(0xE3FE); // consume the $55 response
    assert_eq!(bus.read(0xE3FF) & 0x01, 0x00, "no data pending after $55 consumed");
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
    let cfg = Config::from_toml_str(toml).expect("valid TOML must parse");
    assert!(matches!(cfg.rom_bank, RomBank::Video), "rom_bank must be video");
    assert!(matches!(cfg.rtc_policy, RtcPolicy::Host), "rtc_policy must be host");
    assert_eq!(cfg.open_bus.value, 0xEA, "open_bus must be 0xEA");
    // Verify the Bus initialises without panic using those settings.
    let _bus = blank_bus(cfg);
}
