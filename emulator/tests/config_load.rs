//! Integration tests for `Config::load_from_file_path`, the real-file-on-disk
//! path behind `Config::load()`. `load()` itself reads `std::env::args()` and
//! can't be driven from a test, so these exercise the file-reading/parsing
//! helper directly with real temp files.

use emulator::config::{AciaVariant, Config, CpuSubtype, MmuPowerOnFill, RomBank, RtcPolicy};

/// Build a unique path under the OS temp dir for this test, so parallel test
/// threads (all sharing one process id) don't collide on the same file.
fn temp_path(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("pc6502-config-load-test-{}-{}.toml", std::process::id(), label))
}

#[test]
fn missing_file_falls_back_to_defaults() {
    let path = temp_path("missing");
    // Ensure it really doesn't exist.
    let _ = std::fs::remove_file(&path);

    let cfg = Config::load_from_file_path(path.to_str().unwrap());

    assert_eq!(cfg.disk_image, None);
    assert_eq!(cfg.rom_hex, None);
    assert_eq!(cfg.config_path, None);
}

#[test]
fn minimal_config_sets_disk_and_rom_paths() {
    let path = temp_path("minimal");
    std::fs::write(&path, "disk_image = \"/tmp/test.img\"\nrom_hex = \"/tmp/test.hex\"\n").unwrap();

    let cfg = Config::load_from_file_path(path.to_str().unwrap());

    assert_eq!(cfg.disk_image, Some("/tmp/test.img".to_string()));
    assert_eq!(cfg.rom_hex, Some("/tmp/test.hex".to_string()));
    assert_eq!(cfg.config_path, Some(path.to_str().unwrap().to_string()));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn full_config_round_trips_all_top_level_keys_and_open_bus() {
    let path = temp_path("full");
    let toml = r#"
cpu_subtype = "cmos65c02"
cpu_hz = 2000000
mmu_power_on_fill = "zero"
rom_bank = "video"
shadow_addr_low = false
io_rom_always = true
acia_variant = "w65c51n"
acia_cts_default = false
disk_image = "/tmp/full-disk.img"
rom_hex = "/tmp/full-rom.hex"
rtc_policy = "fixed"
rtc_epoch = 999999999

[open_bus]
value = 255
"#;
    std::fs::write(&path, toml).unwrap();

    let cfg = Config::load_from_file_path(path.to_str().unwrap());

    assert!(matches!(cfg.cpu_subtype, CpuSubtype::Cmos65c02));
    assert_eq!(cfg.cpu_hz, 2_000_000);
    assert!(matches!(cfg.mmu_power_on_fill, MmuPowerOnFill::Zero));
    assert!(matches!(cfg.rom_bank, RomBank::Video));
    assert_eq!(cfg.shadow_addr_low, false);
    assert_eq!(cfg.io_rom_always, true);
    assert!(matches!(cfg.acia_variant, AciaVariant::W65c51n));
    assert_eq!(cfg.acia_cts_default, false);
    assert_eq!(cfg.disk_image, Some("/tmp/full-disk.img".to_string()));
    assert_eq!(cfg.rom_hex, Some("/tmp/full-rom.hex".to_string()));
    assert!(matches!(cfg.rtc_policy, RtcPolicy::Fixed));
    assert_eq!(cfg.rtc_epoch, 999_999_999);
    assert_eq!(cfg.open_bus.value, 255);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn malformed_toml_falls_back_to_defaults() {
    let path = temp_path("malformed");
    std::fs::write(&path, "this is not valid toml === [[[\n").unwrap();

    let cfg = Config::load_from_file_path(path.to_str().unwrap());

    assert_eq!(cfg.disk_image, None);
    assert_eq!(cfg.rom_hex, None);
    assert_eq!(cfg.config_path, None);

    let _ = std::fs::remove_file(&path);
}
