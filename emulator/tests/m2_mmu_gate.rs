// Gate test for WI-M2: MMU — 64 tasks, BIOS INITPAGES, task switching.
//
// Acceptance: INITPAGES completes; $EFE4 shows task-0 + enable; banner persists.
//
// Full implementation in WI-M2.

#[test]
#[ignore = "requires rom.hex — implement in WI-M2"]
fn m2_initpages_and_map_roundtrip() {
    // TODO(WI-M2): after BIOS INITPAGES: read($EFE4) & 0xBF == 0x80
    // TODO(WI-M2): write 16 known bytes to $EFD0–$EFDF; read back identical
    // TODO(WI-M2): task-1 $C000 → physical $10000
    // TODO(WI-M2): task-0 $1234 → physical $01234
    // TODO(WI-M2): banner still printed after INITPAGES
    // TODO(WI-M2): write $FF to $EFE0; read($EFE4) & 0x3F == 0x3F
}
