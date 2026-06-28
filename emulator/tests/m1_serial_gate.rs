// Gate test for WI-M1: CPU core, flat bus, ACIA serial output.
//
// Acceptance: Supermon '>' prompt on stdout after reset within 10M cycles;
// 'G F000\r' re-emits the banner without hanging.
//
// Full implementation in WI-M1.

#[test]
#[ignore = "requires rom.hex — implement in WI-M1"]
fn m1_supermon_prompt() {
    // TODO(WI-M1): run emulator until '>' appears on serial stdout (≤ 10M cycles)
    // TODO(WI-M1): inject 'G F000\r'; assert banner re-emits; assert no hang
}
