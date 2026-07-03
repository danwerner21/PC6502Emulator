# PC6502 Emulator — Quick Start

## Prerequisites

- **Rust toolchain** (edition 2021; any stable ≥ 1.56 works):
  ```
  rustup update stable
  ```
- No system libraries required beyond the Rust standard library.
- `ca65`, `ld65`, `srec_cat` — only needed if you want to rebuild the firmware
  from source in `PC6502_firmware_source/`. Pre-built artifacts are already
  present in `disk_image/` and `PC6502_firmware_source/rom.hex`.
- **On the `/mnt/fileserver` CIFS mount only**, one-time per host/checkout:
  ```bash
  bash emulator/scripts/ensure-local-cargo-target.sh
  ```
  Without this, `cargo run`/`build`/`test` can fail intermittently with
  `Invalid argument (os error 22)`. See
  [Troubleshooting](#troubleshooting) for why this step (still) can't be
  automated away, and why it's a one-time thing rather than a per-session
  export.

---

## Building

```bash
cd emulator/
cargo build --release
```

Run in-place without installing:

```bash
cargo run --release -- --config config/default.toml
```

---

## ROM (`rom.hex`)

The emulator needs an 8 KiB ROM image split into Base and VIDEO banks.
The pre-built file lives at:

```
PC6502_firmware_source/rom.hex   ← canonical
emulator/disk_image/rom.hex      ← identical copy (convenience)
```

The emulator finds the ROM two ways (in order):

1. `PC6502_ROM_HEX` environment variable
2. `rom_hex = "..."` key in the config file passed via `--config`

If neither is set the ROM is blank (`$FF`-filled) — the CPU will fetch
`$FFFF` as the reset vector and spin; no useful output results.

---

## Disk Image (`disk.img`)

A raw flat binary disk image is required for the XT-IDE controller (M3+).
The pre-built image lives at:

```
emulator/disk_image/disk.img   (8,519,680 bytes, 16,640 sectors)
```

Point the emulator at it via:

1. `PC6502_DISK_IMG` environment variable, or
2. `disk_image = "disk_image/disk.img"` in the config file

Without a disk image the XT-IDE controller returns zeros for every sector
read. The emulator starts and the Base ROM reaches Supermon, but `B` (IDE
boot) hangs waiting for a real sector.

---

## Running

**Minimal — Base ROM only, reach Supermon prompt:**

```bash
PC6502_ROM_HEX=../PC6502_firmware_source/rom.hex \
  cargo run --release -- --config config/default.toml
```

Serial output appears on stdout. The Base ROM prints the Supermon banner
and prompt `>` then blocks on ACIA RX input; type commands via stdin
(piped or interactive — the emulator polls `$EF85` RDRF).

When stdin is a real terminal, the emulator puts it in raw mode and feeds
each keypress to ACIA RX immediately — no Enter needed. **Ctrl-C exits**
the emulator (it's delivered as a plain byte rather than a signal, since raw
mode disables the terminal's own SIGINT handling); **Ctrl-D** also exits.
When piping input, send it after the boot banner/prompt has appeared —
bytes that arrive before the ROM's own serial init (`SERIALINIT`) completes
are cleared by its ACIA reset, same as a real UART would drop bytes that
arrive before it's initialized.

**Full boot — VIDEO ROM + disk, reach DOS/65 `A>` prompt:**

```bash
PC6502_ROM_HEX=../PC6502_firmware_source/rom.hex \
PC6502_DISK_IMG=disk_image/disk.img \
  cargo run --release -- --config my.toml
```

where `my.toml` sets `rom_bank = "video"` (see §Configuration below).
The VIDEO ROM runs the IDE loader automatically; after ~1–2 seconds the
DOS/65 banner and `A>` appear on stdout.

---

## Configuration

Copy `config/default.toml` and edit as needed:

| Key | Default | When to change |
|---|---|---|
| `rom_bank` | `"base"` | Set `"video"` for unattended DOS/65 boot (M3+) |
| `cpu_subtype` | `"nmos_6502"` | Change to `"cmos_65c02"` if hardware confirms a 65C02 |
| `rtc_policy` | `"host"` | `"fixed"` + `rtc_epoch` for reproducible test epochs |
| `open_bus.value` | `0xEA` | `0xFF` is the other common candidate; never `0x00` |
| `disk_image` | *(unset)* | Set to `"disk_image/disk.img"` (or use env var) |
| `rom_hex` | *(unset)* | Set to path of `rom.hex` (or use env var) |

Pass your config with `--config path/to/my.toml`. Keys absent from the
file fall back to compiled defaults.

---

## Running the Gate Tests

```bash
cd emulator/
cargo test
```

- **M1** resolves `rom.hex` by searching upward from `CARGO_MANIFEST_DIR`
  for `PC6502_firmware_source/rom.hex`, so it finds the fixture
  unmodified from both the main checkout and a worktree. **M2** still
  walks a fixed three directories up, which only reaches the fixture
  from a worktree; from the main checkout it fails its assertion unless
  `PC6502_ROM_HEX` is set. Neither test skips silently if the fixture
  can't be found — both fail with an assertion naming the path they
  tried. Set `PC6502_ROM_HEX` to override the lookup for either.
- **M3, M4, M5** integration tests using real artifacts need the disk
  image:
  ```bash
  PC6502_DISK_IMG=disk_image/disk.img cargo test
  ```
- **M4 real-boot test** (`m4_real_boot_far_call_and_sim_init`) runs the
  full VIDEO ROM → 60-sector load → DOS/65 cold boot path. Expect ~60–90
  seconds on a 2 GHz host.
- **`m5_com_load`** runs with the stock `disk.img` (24 `.COM` files
  present); a `.COM` must be named in the test or passed via env var.
- Worktree builds: `disk_image/` exists only in the main checkout. Set
  `PC6502_DISK_IMG` to an absolute path when running from a worktree.

---

## Known Quirks

**DOS/65 serial output goes through the banked dispatcher.**  
Output from DOS/65 (not the ROM) reaches the ACIA via far-call `$FFF0`
→ task-1 dispatcher `$C000`. The emulator must correctly preserve the
task-1 driver copy at physical `$10000–$115B0` across far calls. If the
ACIA shows ROM banner text but nothing after `A>`, check that the task-1
copy succeeded (physical `$10000` non-zero) and that the `CONSOLE` byte
at zero-page `$3A` is `$04` (serial output function base).

**`pcdos65.s19` is a provisioning reference, not a test input.**  
The S-record and `disk.img` may come from different build runs. The
S-record documents the intended staging layout (loader at $0800, kernel
at $1000–$3070, driver at $4000–$55B0); `disk.img` is the authoritative
runtime artifact. Do not assume they are byte-for-byte consistent.

**CP/M directory starts at LBA 256 (0x0100), not LBA 60.**  
LBAs 0–59 are the boot area. LBAs 60–255 are the tail of the reserved
system tracks ($E5-filled). The actual directory with 29 files is at
LBA 256. Tools that scan from LBA 0 for CP/M signatures will miss it.

**Drive B exceeds the committed disk image.**  
`disk.img` covers drive A only (16,640 sectors). Drive B (LBAs
$4100–$81FF) is beyond the image end. In-memory writes auto-extend but
are not persisted. Full drive-B testing requires a larger image.

---

## Troubleshooting

The emulator always prints four lines to stderr on startup — config path,
ROM (path + bank, or a blank-ROM warning), disk (path + sector count, or
"none"), and the reset vector fetched from ROM. Check these first; they
catch most misconfiguration before it looks like a hang.

**"Nothing appears on stdout."**  
Check stderr for the `ROM: blank ($FF-filled)` warning. That means
`PC6502_ROM_HEX` is unset and the config has no `rom_hex` key, so the CPU
is executing an all-`$FF` ROM and will never produce output. Confirm
`PC6502_ROM_HEX` is set and the path is correct relative to the current
working directory (`emulator/` when using `cargo run`).

**"Emulator hangs after the banner."**  
Likely waiting on the disk. Check the `Disk:` startup line on stderr —
if it says "none", confirm `PC6502_DISK_IMG` is set and `disk.img`
exists at that path.

**"No output at all, not even the ROM banner."**  
Run with `--debug` and watch the heartbeat lines on stderr: if `PC` is
stuck near `$FFFF` (or never advances), the ROM is blank; if `PC` cycles
through the same small range of addresses, the CPU is looping (e.g.
waiting on a peripheral that never responds).

```bash
PC6502_ROM_HEX=../PC6502_firmware_source/rom.hex \
  cargo run --release -- --config config/default.toml --debug
```

**"DOS/65 output stops after the banner."**  
See Known Quirks above: the zero-page `$3A` dispatch byte must be `$04`,
and the task-1 driver copy at physical `$10000` must be non-zero.

**`cargo run`/a freshly-built binary fails with `error: could not
execute process ... (never executed)` / `Invalid argument (os error
22)`.**  
This is a CIFS quirk on the `/mnt/fileserver` mount
(`cache=strict,actimeo=1`), not a bug in the emulator, and not a
regression — it reproduces from a clean, unmodified checkout on
lasever04. It shows up as a transient `EINVAL` when a process opens a
binary shortly after cargo last wrote to the `target/` directory
(building or just re-checking freshness); a `cp` of a freshly-written
binary can hit the same `EINVAL`. **It is flaky, not deterministic** —
in testing, the exact same command/binary pair flipped between failing
and succeeding across repeated attempts, including after the build had
gone stale for tens of seconds and after a `sync`. Do not trust a
single success or failure as conclusive, and do not rely on a fixed
delay or a `cargo build` / direct-exec split as "the fix" — both were
observed to still fail intermittently.

**Default fix: `target-dir` is redirected to local (non-CIFS) disk
automatically**, via a generated `.cargo/config.toml` (see
Prerequisites above) — no environment variable to export, and nothing
to remember each session. Run once per host/checkout:

```bash
bash emulator/scripts/ensure-local-cargo-target.sh
```

This writes `emulator/.cargo/config.toml` with `build.target-dir`
pointing at `${XDG_CACHE_HOME:-$HOME/.cache}/pc6502-emulator/cargo-target-<hash>`,
where `<hash>` is derived from the crate's own absolute path so each
git worktree under `worktrees/` gets its own target-dir instead of
contending for one shared build lock. `$HOME` on lasever04 is local
ext4, not CIFS, so the compiled binary's exec never touches the CIFS
client's write-back/cache path. Confirmed reliable across repeated
forced rebuilds in testing (0 `EINVAL` failures after the CIFS-only
default had already reproduced the failure). Source files are still
read from `/mnt/fileserver` — only compiled artifacts move — so this
doesn't require relocating the checkout. The `cargo test` gate suite
(M1–M6) uses the same target-dir and was run repeatedly with no
regressions.

Why a one-time script run and not a fully zero-touch default: Cargo's
`.cargo/config.toml` does not expand `~` or `${VAR}` in path values
(verified directly against the cargo version installed here), so no
single path committed to version control can be correct for every
user/host — it would either be wrong for a different user or, if
shared, permission-hostile. The generated config file is git-ignored
(`emulator/.gitignore`) for the same reason. If `cargo run` ever fails
with this error again, re-run the script above (idempotent, safe to
re-run) and confirm `emulator/.cargo/config.toml` exists and points at
a writable path.
