// Gate test for WI-M4: DOS/65 cold boot to prompt.
//
// Acceptance: 'DOS/65' and 'A>' on serial console; 'A:\r' echoes without hang.
//
// Full implementation in WI-M4.

#[test]
#[ignore = "requires rom.hex and disk.img — implement in WI-M4"]
fn m4_dos65_cold_boot_prompt() {
    // TODO(WI-M4): physical $B800–$D870 non-zero after task-0 copy
    // TODO(WI-M4): physical $10000–$11FFF non-zero after task-1 copy
    // TODO(WI-M4): stdout contains "DOS/65" then "A>"
    // TODO(WI-M4): inject "A:\r"; assert echo; no timeout or fault
    // TODO(WI-M4): SIM device-init: no absent-device failure halts execution
}
