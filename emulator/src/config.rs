use serde::Deserialize;

/// Which 6502 CPU variant to emulate (OQ-R0.1).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CpuSubtype {
    Nmos6502,
    Cmos65c02,
}

impl Default for CpuSubtype {
    fn default() -> Self {
        CpuSubtype::Nmos6502
    }
}

/// Which K1 ROM bank is active at power-on (OQ-R0.4).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RomBank {
    Base,
    Video,
}

impl Default for RomBank {
    fn default() -> Self {
        RomBank::Base
    }
}

/// ACIA hardware variant (OQ-R1.1).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AciaVariant {
    R6551,
    W65c51n,
}

impl Default for AciaVariant {
    fn default() -> Self {
        AciaVariant::R6551
    }
}

/// Open-bus byte policy for unmapped reads (OQ-R0.5).
/// Value is returned for $EFA0–$EFCF, $EFF0–$EFFF, unassigned MMU offsets,
/// and physical pages above $7F that are not I/O or ROM.
#[derive(Debug, Clone, Deserialize)]
pub struct OpenBusPolicy {
    /// The byte value returned for unmapped reads. $EA (NOP) is a reasonable
    /// first choice (R-6); $FF is another common candidate.
    pub value: u8,
}

impl Default for OpenBusPolicy {
    fn default() -> Self {
        OpenBusPolicy { value: 0xEA }
    }
}

/// Fill strategy for MMU map SRAM at power-on (OQ-R0.3).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MmuPowerOnFill {
    Zero,
    Random,
    Fixed(u8),
}

impl Default for MmuPowerOnFill {
    fn default() -> Self {
        MmuPowerOnFill::Random
    }
}

/// RTC time-source policy (OQ for rtc_policy in WI-M6).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RtcPolicy {
    Host,
    Fixed,
    Epoch,
}

impl Default for RtcPolicy {
    fn default() -> Self {
        RtcPolicy::Host
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// OQ-R0.1 — CPU part: nmos_6502 or cmos_65c02.
    #[serde(default)]
    pub cpu_subtype: CpuSubtype,

    /// OQ-R0.2 — CPU oscillator frequency in Hz. Not used for timing accuracy,
    /// exposed for future use.
    #[serde(default = "default_cpu_hz")]
    pub cpu_hz: u64,

    /// OQ-R0.3 — MMU map SRAM contents at power-on: zero, random, or fixed(n).
    #[serde(default)]
    pub mmu_power_on_fill: MmuPowerOnFill,

    /// OQ-R0.4 — Active K1 ROM bank at power-on: base or video.
    #[serde(default)]
    pub rom_bank: RomBank,

    /// OQ-R0.5 — Open-bus policy for unmapped reads.
    #[serde(default)]
    pub open_bus: OpenBusPolicy,

    /// OQ-R0.6 — Shadow-address strap setting. true = low-page (firmware-compatible default).
    #[serde(default = "default_true")]
    pub shadow_addr_low: bool,

    /// OQ-R0.7 — When true, I/O and ROM decode always takes precedence over MMU
    /// mappings at the corresponding physical pages. Unconfirmed on hardware;
    /// set false to use identity-map-only behavior.
    #[serde(default)]
    pub io_rom_always: bool,

    /// OQ-R1.1 — ACIA variant: r6551 or w65c51n.
    #[serde(default)]
    pub acia_variant: AciaVariant,

    /// OQ-R1.4 — CTS signal default. true = CTS asserted (high), which is
    /// firmware-compatible per board jumper belief. A wrong default deadlocks TX.
    #[serde(default = "default_true")]
    pub acia_cts_default: bool,

    /// G-2 (plan-review gap) — Path to raw flat disk image for XT-IDE.
    #[serde(default)]
    pub disk_image: Option<String>,

    /// Path to rom.hex Intel HEX file.
    #[serde(default)]
    pub rom_hex: Option<String>,

    /// RTC clock policy: host, fixed, or epoch.
    #[serde(default)]
    pub rtc_policy: RtcPolicy,

    /// Unix timestamp (seconds since 1970-01-01 UTC) used by `fixed` and `epoch`
    /// RTC policies.  Default: 2025-01-01 00:00:00 UTC.
    #[serde(default = "default_rtc_epoch")]
    pub rtc_epoch: u64,

    /// Path passed via `--config`, recorded for startup diagnostics.
    /// `None` means compiled defaults (no `--config` flag given). Not itself
    /// a config file key.
    #[serde(skip)]
    pub config_path: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            cpu_subtype: CpuSubtype::Nmos6502,
            cpu_hz: 1_000_000,
            mmu_power_on_fill: MmuPowerOnFill::Random,
            rom_bank: RomBank::Base,
            open_bus: OpenBusPolicy::default(),
            shadow_addr_low: true,
            io_rom_always: false,
            acia_variant: AciaVariant::R6551,
            acia_cts_default: true,
            disk_image: None,
            rom_hex: None,
            rtc_policy: RtcPolicy::Host,
            rtc_epoch: default_rtc_epoch(),
            config_path: None,
        }
    }
}

fn default_cpu_hz() -> u64 {
    1_000_000
}

fn default_true() -> bool {
    true
}

fn default_rtc_epoch() -> u64 {
    1_735_689_600 // 2025-01-01 00:00:00 UTC
}

impl Config {
    pub fn load() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let path = args.windows(2).find(|w| w[0] == "--config").map(|w| w[1].as_str().to_string());
        let mut cfg = match path {
            Some(p) => Self::load_from_file_path(&p),
            None => Config::default(),
        };

        // Env vars override the config file (see QUICKSTART.md ROM/Disk sections).
        if let Ok(v) = std::env::var("PC6502_ROM_HEX") {
            cfg.rom_hex = Some(v);
        }
        if let Ok(v) = std::env::var("PC6502_DISK_IMG") {
            cfg.disk_image = Some(v);
        }

        cfg
    }

    /// Read and parse a config file at `path`. Errors are reported on stderr
    /// and fall back to compiled defaults rather than silently pretending the
    /// file loaded (`config_path` stays `None` on failure so
    /// `print_startup_diagnostics` reports "compiled defaults", not the path).
    /// Exposed as `pub` (rather than folded into `load()`) so integration
    /// tests can exercise real-file parsing without faking `argv`.
    pub fn load_from_file_path(path: &str) -> Self {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Config: ERROR — cannot read '{}': {} (using compiled defaults)", path, e);
                eprintln!("Config: cwd is {}", std::env::current_dir().unwrap_or_default().display());
                return Config::default();
            }
        };
        let mut cfg: Config = match toml::from_str(&text) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Config: ERROR — failed to parse '{}': {} (using compiled defaults)", path, e);
                return Config::default();
            }
        };
        cfg.config_path = Some(path.to_string());
        cfg
    }

    /// Parse a TOML string, propagating parse errors to the caller.
    pub fn from_toml_str(s: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(s)
    }
}
