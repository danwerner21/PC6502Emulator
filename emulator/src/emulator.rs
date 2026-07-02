use crate::bus::Bus;
use crate::config::{Config, RomBank};
use crate::cpu::Cpu;
use crate::disk::DiskImage;
use crate::rom::Rom;
use std::sync::mpsc;
use std::thread;

/// Top-level machine: owns the CPU, bus, and all peripherals.
pub struct Machine {
    pub cpu: Cpu,
    pub bus: Bus,
    /// When true, `run()` prints a one-line CPU status to stderr every 1000 cycles.
    pub debug: bool,
    diag: StartupDiag,
}

/// How the machine ended up configured, captured in `new()` for the startup
/// diagnostics `run()` prints — `Config` itself is consumed before `run()`.
struct StartupDiag {
    config_path: Option<String>,
    rom_hex_path: Option<String>,
    rom_loaded: bool,
    rom_bank: RomBank,
    disk_path: Option<String>,
    disk_sectors: Option<u32>,
    /// The configured disk path, set only when `DiskImage::load()` failed for it.
    disk_error: Option<String>,
}

fn rom_bank_str(bank: &RomBank) -> &'static str {
    match bank {
        RomBank::Base => "base",
        RomBank::Video => "video",
    }
}

/// Render the last ACIA byte emitted for the `--debug` heartbeat line.
fn format_last_acia(byte: Option<u8>) -> String {
    match byte {
        None => "-".to_string(),
        Some(b) if (0x20..=0x7E).contains(&b) => (b as char).to_string(),
        Some(b) => format!("\\x{:02X}", b),
    }
}

/// Restores the terminal from raw mode when dropped, including during panic
/// unwinding, so a crash never leaves the user's shell in raw mode.
struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

/// Spawn a background thread that reads stdin byte-by-byte and forwards each
/// byte over the returned channel. Used by `run()` to feed keystrokes into
/// ACIA RX without blocking the CPU loop on stdin I/O.
fn spawn_stdin_reader() -> mpsc::Receiver<u8> {
    let (tx, rx) = mpsc::channel::<u8>();
    thread::spawn(move || {
        use std::io::Read;
        let mut stdin = std::io::stdin();
        let mut byte = [0u8; 1];
        loop {
            match stdin.read(&mut byte) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if tx.send(byte[0]).is_err() {
                        break;
                    }
                }
            }
        }
    });
    rx
}

impl Machine {
    pub fn new(cfg: Config) -> Self {
        let rom_hex_path = cfg.rom_hex.clone();
        let rom_load_result = cfg
            .rom_hex
            .as_ref()
            .map(|path| Rom::load_hex(path, cfg.rom_bank.clone()));
        let rom_loaded = matches!(rom_load_result, Some(Ok(_)));
        let rom = match rom_load_result {
            Some(Ok(r)) => r,
            _ => Rom::blank(cfg.rom_bank.clone()),
        };

        let disk_path = cfg.disk_image.clone();
        let disk_result = cfg
            .disk_image
            .as_ref()
            .map(|p| DiskImage::load(p).map(|d| (p.clone(), d)));
        let (disk, disk_sectors, disk_error) = match disk_result {
            Some(Ok((_, d))) => {
                let sectors = d.sector_count();
                (Some(d), Some(sectors), None)
            }
            Some(Err(_)) => (None, None, cfg.disk_image.clone()),
            None => (None, None, None),
        };

        let diag = StartupDiag {
            config_path: cfg.config_path.clone(),
            rom_hex_path,
            rom_loaded,
            rom_bank: cfg.rom_bank.clone(),
            disk_path,
            disk_sectors,
            disk_error,
        };

        let bus = Bus::new(&cfg, rom, disk);
        let cpu = Cpu::new();
        Machine { cpu, bus, debug: false, diag }
    }

    /// Construct directly from a CPU and bus, bypassing config-based ROM/disk
    /// loading. For tests that build a synthetic ROM or bus in-memory rather
    /// than loading one from a file; startup diagnostics report empty/blank
    /// for a machine built this way.
    pub fn from_parts(cpu: Cpu, bus: Bus) -> Self {
        Machine {
            cpu,
            bus,
            debug: false,
            diag: StartupDiag {
                config_path: None,
                rom_hex_path: None,
                rom_loaded: false,
                rom_bank: RomBank::Base,
                disk_path: None,
                disk_sectors: None,
                disk_error: None,
            },
        }
    }

    /// Execute one instruction.  Returns cycles consumed.
    pub fn step_one(&mut self) -> u32 {
        // Safety: read and write callbacks are called sequentially by the CPU,
        // never concurrently, so aliasing the bus pointer is sound.
        let bus: *mut Bus = &mut self.bus;
        let cpu = &mut self.cpu;
        cpu.step(
            |addr| unsafe { (*bus).read(addr) },
            |addr, val| unsafe { (*bus).write(addr, val) },
        )
    }

    /// Print config/ROM/disk/reset-vector diagnostics to stderr. Always runs
    /// (independent of `--debug`) so a blank ROM or missing disk is visible
    /// immediately instead of looking like a silent hang.
    fn print_startup_diagnostics(&self) {
        eprintln!(
            "Config: {}",
            self.diag.config_path.as_deref().unwrap_or("compiled defaults")
        );
        match (self.diag.rom_hex_path.as_deref(), self.diag.rom_loaded) {
            (Some(path), true) => eprintln!(
                "ROM: {} (bank: {})",
                path,
                rom_bank_str(&self.diag.rom_bank)
            ),
            (Some(path), false) => eprintln!(
                "ROM: WARNING — configured as \"{}\" but could not open (using blank $FF-filled ROM)",
                path
            ),
            (None, _) => {
                eprintln!("ROM: blank ($FF-filled) — set PC6502_ROM_HEX or rom_hex in config")
            }
        }
        match (self.diag.disk_path.as_deref(), self.diag.disk_sectors) {
            (Some(path), Some(sectors)) => eprintln!("Disk: {} ({} sectors)", path, sectors),
            (Some(_), None) => {
                let path = self.diag.disk_error.as_deref().unwrap_or("<unknown>");
                let cwd = std::env::current_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| "<unknown>".to_string());
                eprintln!(
                    "Disk: ERROR — configured as \"{}\" but could not open (check path is relative to cwd: {})",
                    path, cwd
                );
            }
            (None, _) => eprintln!("Disk: none — XT-IDE returns zeros"),
        }
        eprintln!("Reset vector: ${:04X}", self.cpu.pc);
    }

    /// Run with RESET, feed stdin into ACIA RX, and drain ACIA output to
    /// stdout in a loop. Ctrl-C and Ctrl-D from stdin exit the loop cleanly.
    pub fn run(&mut self) {
        {
            let bus = &mut self.bus;
            self.cpu.reset(|addr| bus.read(addr));
        }
        self.print_startup_diagnostics();

        // Raw mode delivers each keypress as soon as it's typed, without
        // waiting for Enter. It also disables the terminal's own SIGINT
        // handling, so Ctrl-C arrives as a plain byte (0x03) below instead
        // of killing the process — that's why we check for it explicitly.
        // If stdin isn't a real TTY (e.g. piped input) this fails and we
        // fall back to plain reads; piped bytes still arrive without needing
        // raw mode.
        let _raw_mode_guard = crossterm::terminal::enable_raw_mode()
            .ok()
            .map(|_| RawModeGuard);

        let stdin_rx = spawn_stdin_reader();

        let mut total: u64 = 0;
        let mut next_debug_at: u64 = 1000;
        let mut last_acia_byte: Option<u8> = None;

        loop {
            total += self.step_one() as u64;

            // Drain buffered serial output to stdout
            let out = self.bus.acia_mut().drain_output();
            if !out.is_empty() {
                use std::io::Write;
                let _ = std::io::stdout().write_all(&out);
                let _ = std::io::stdout().flush();
                last_acia_byte = out.last().copied();
            }

            // Feed keystrokes typed since the last cycle into ACIA RX.
            let mut exit_requested = false;
            for b in stdin_rx.try_iter() {
                if b == 0x03 || b == 0x04 {
                    // Ctrl-C / Ctrl-D: exit cleanly rather than delivering
                    // to the guest.
                    exit_requested = true;
                    break;
                }
                self.bus.acia_mut().inject_rx(b);
            }
            if exit_requested {
                break;
            }

            if self.debug {
                while total >= next_debug_at {
                    eprintln!(
                        "[{:>7}] PC={:04X} A={:02X} X={:02X} Y={:02X} SP={:02X}  last_acia='{}'",
                        next_debug_at,
                        self.cpu.pc,
                        self.cpu.a,
                        self.cpu.x,
                        self.cpu.y,
                        self.cpu.sp,
                        format_last_acia(last_acia_byte)
                    );
                    next_debug_at += 1000;
                }
            }
        }
    }

    /// Run up to `max_cycles` after RESET, returning total cycles executed.
    pub fn run_until_cycles(&mut self, max_cycles: u64) -> u64 {
        {
            let bus = &mut self.bus;
            self.cpu.reset(|addr| bus.read(addr));
        }
        let mut total: u64 = 0;
        while total < max_cycles {
            total += self.step_one() as u64;
        }
        total
    }
}
