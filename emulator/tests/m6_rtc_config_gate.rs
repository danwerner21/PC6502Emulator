// Gate test for WI-M6: RTC model, config-file boot, configuration hardening.
//
// Acceptance: RTC read returns plausible date; emulator starts from custom config.
//
// Full implementation in WI-M6.

#[test]
#[ignore = "requires rom.hex, disk.img, and rtc implementation — implement in WI-M6"]
fn m6_rtc_and_config_boot() {
    // TODO(WI-M6): rtc_policy=host: DOS time/date returns year 2020–2040
    // TODO(WI-M6): firmware write sequence $02/$00/$00/$01/$05/$04: no fault; clock advances
    // TODO(WI-M6): rtc_policy=fixed with known epoch: returned date matches
    // TODO(WI-M6): CH375 C: access: failure returned; no crash; A> returns
    // TODO(WI-M6): Multi-I/O $AA → $55; init does not hang
    // TODO(WI-M6): read($EFA0) == configured open_bus value
    // TODO(WI-M6): --config with rom_bank=video, open_bus=0xEA, rtc_policy=host: boot succeeds
}
