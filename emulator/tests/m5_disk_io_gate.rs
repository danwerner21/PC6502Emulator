// Gate test for WI-M5: DOS/65 disk read/write, directory listing.
//
// Acceptance: 'DIR A:' lists CP/M directory entries from disk image.
//
// Full implementation in WI-M5.

#[test]
#[ignore = "requires rom.hex and disk.img — implement in WI-M5"]
fn m5_dir_listing_and_disk_io() {
    // TODO(WI-M5): inject "DIR A:\r"; assert CP/M directory entries in stdout
    // TODO(WI-M5): load valid .COM; assert output, no hang
    // TODO(WI-M5): write then read-back small file: byte-identical
    // TODO(WI-M5): drive E access: failure returned; no crash; no hang
    // TODO(WI-M5): bad-sector injection: "BAD SECTOR" in stdout; Return → "A>"
    // TODO(WI-M5): B-drive write does not corrupt A-range sectors (LBA 0 unchanged)
}
