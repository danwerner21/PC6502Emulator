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

- **M1 and M2** tests resolve `rom.hex` by walking three directories up
  from `CARGO_MANIFEST_DIR` to `PC6502_firmware_source/rom.hex`. They
  will skip gracefully if the file is absent; set `PC6502_ROM_HEX` to
  override.
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
