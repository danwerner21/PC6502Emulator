// Gate test for WI-M3: XT-IDE controller, disk image, VIDEO ROM boot.
//
// Acceptance: VIDEO bank reset vector = $F000; 60 sector transfers complete; PC = $0800.
//
// Full implementation in WI-M3.

#[test]
#[ignore = "requires rom.hex and disk.img — implement in WI-M3"]
fn m3_video_boot_and_60_sectors() {
    // TODO(WI-M3): rom_bank=video: reset vector returns $F000
    // TODO(WI-M3): probe writes $FF/$00 to $E300–$E330: no crash; LBA 0 unchanged
    // TODO(WI-M3): SET FEATURES $EF: BSY clears; DRQ and ERR absent
    // TODO(WI-M3): 60 sector reads succeed; sector transfer counter reaches 60
    // TODO(WI-M3): CPU PC == $0800 after 60th transfer
}
