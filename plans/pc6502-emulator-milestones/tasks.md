---
plan_slug: pc6502-emulator-milestones
phase: tasks
rig: PC6502Emulator
rig_root: /mnt/fileserver/Vintage/Projects/PC6502Emulator
artifact_root: /mnt/fileserver/Vintage/Projects/PC6502Emulator/plans
requirements_file: /mnt/fileserver/Vintage/Projects/PC6502Emulator/plans/pc6502-emulator-milestones/requirements.md
implementation_plan_file: /mnt/fileserver/Vintage/Projects/PC6502Emulator/plans/pc6502-emulator-milestones/plan.md
status: approved
created_at: 2026-06-28T15:00:00Z
updated_at: 2026-06-28T16:00:00Z
---

# Task Plan: PC6502 Emulator — Six-Milestone Build

## Summary

Seven sequential implementation beads in one convoy. Each bead corresponds to one
build increment from the approved implementation plan; each is gated by observable
serial output from the actual `rom.hex` and DOS/65 disk images — not by unit tests
alone.

| Key | Title | Gate observable |
|-----|-------|-----------------|
| setup | WI-SETUP: Cargo workspace scaffold | `cargo check` passes |
| m1 | WI-M1: CPU core, flat bus, ACIA, ROM loader | Supermon `>` on serial |
| m2 | WI-M2: MMU — 64-task map, address translation | INITPAGES; `$EFE4` readback |
| m3 | WI-M3: XT-IDE controller, disk image, VIDEO ROM boot | 60 sectors; PC=`$0800` |
| m4 | WI-M4: DOS/65 cold boot, absent-device stubs | `DOS/65` and `A>` on serial |
| m5 | WI-M5: DOS/65 disk read/write, DIR listing | `DIR A:` lists entries |
| m6 | WI-M6: RTC model, config-file boot, hardening | RTC date; config-file boot |

All beads run sequentially; each milestone depends strictly on the previous.
Regression policy: once a milestone gate test passes it must stay green through all
subsequent milestones.

## Created Beads

| Key | Bead ID | Title |
|-----|---------|-------|
| impl-convoy | pe-b6j | PC6502 Emulator — Implementation Convoy (M1–M6) |
| setup | pe-u30 | WI-SETUP: Cargo workspace scaffold |
| m1 | pe-5yg | WI-M1: CPU core, flat bus, ACIA, ROM loader |
| m2 | pe-xr4 | WI-M2: MMU — 64-task map, address translation |
| m3 | pe-1rx | WI-M3: XT-IDE controller, disk image, VIDEO ROM boot |
| m4 | pe-tpy | WI-M4: DOS/65 cold boot, absent-device stubs |
| m5 | pe-076 | WI-M5: DOS/65 disk read/write, DIR listing |
| m6 | pe-kc6 | WI-M6: RTC model, config-file boot, configuration hardening |

## Bead Creation Payload

```yaml
target_rig: PC6502Emulator
labels:
  - emulator
  - rust

convoys:
  - key: impl-convoy
    title: "PC6502 Emulator — Implementation Convoy (M1–M6)"
    description: >
      Sequential implementation convoy for the PC6502 software emulator.
      Seven beads (scaffold + six milestones) from workspace setup through
      RTC and configuration hardening. Each bead is gated by running the
      actual rom.hex and DOS/65 disk images and observing specified serial
      output.
    beads:
      - key: setup
        title: "WI-SETUP: Cargo workspace scaffold"
        type: task
        priority: P1
        description: >
          Create the Cargo workspace skeleton at emulator/ in the repository
          root. Produce: emulator/Cargo.toml (workspace root; emulator binary
          crate), emulator/Cargo.lock, emulator/config/default.toml (all P0/P1
          knobs documented with OQ references), and stub skeletons for all src/
          modules listed in the implementation plan (main.rs, config.rs,
          emulator.rs, bus.rs, acia.rs, mmu.rs, xt_ide.rs, disk.rs, rtc.rs,
          rom.rs, peripherals.rs, cpu/mod.rs, cpu/opcodes.rs, cpu/flags.rs)
          plus placeholder files under tests/. Acceptance: cargo check passes
          on empty stubs with no warnings.
        acceptance_criteria:
          - emulator/Cargo.toml exists with [workspace] and [[bin]] sections
          - emulator/config/default.toml exists with all P0/P1 knobs (OQ-R0.1 through OQ-R1.4) documented
          - All src/ module stubs compile; cargo check produces zero errors and zero warnings
          - Placeholder gate test files exist under emulator/tests/

      - key: m1
        title: "WI-M1: CPU core, flat bus, ACIA, ROM loader"
        type: task
        priority: P1
        description: >
          Implement the M1 milestone: 6502 CPU core, flat 64 KiB bus, 6551
          ACIA, and Intel HEX ROM loader. All P0/P1 config knobs must be
          wired. Gate observable: Supermon > prompt on stdout after reset.
          Files to implement: config.rs (Config struct, TOML load via
          --config), rom.rs (load_hex: parse Intel HEX, split Base/VIDEO 4 KiB
          banks), cpu/mod.rs (Cpu::step, RESET/NMI/IRQ/BRK dispatch,
          NMOS/65C02 subtype), cpu/opcodes.rs (all documented 6502 opcodes),
          cpu/flags.rs (flag helpers), bus.rs (flat 64 KiB: RAM $0000-$DFFF,
          I/O $E000-$EFFF, ROM $F000-$FFFF; open-bus; ROM write protection),
          acia.rs (6551 at $EF84-$EF87; TX to stdout; TDRE always set;
          programmed reset; acia_cts_default and acia_variant knobs),
          emulator.rs (Machine struct), main.rs (CLI: --config, ROM load,
          emulator loop), tests/m1_serial_gate.rs (run until > in stdout <=
          10M cycles; inject G F000\r; assert banner re-emits).
          Plan gaps: G-1 (acia_variant knob), G-2 (disk_image placeholder).
        acceptance_criteria:
          - Reset vector fetch at $FFFC-$FFFD returns $00 $F0 (Base bank)
          - BIOS startup SEI/CLD/LDX $FF/TXS completes without CPU fault
          - ACIA init $00->$EF85, $0B->$EF86, $1E->$EF87 does not crash; TDRE bit 4 set
          - Banner characters appear on stdout (TDRE polling exits normally)
          - BRK reaches Supermon; > received on stdout within 10M cycles
          - G F000 re-runs reset path and re-emits banner without hanging
        dependencies:
          - setup

      - key: m2
        title: "WI-M2: MMU — 64-task map, address translation"
        type: task
        priority: P1
        description: >
          Implement the M2 milestone: 64-task MMU with BIOS INITPAGES and
          address translation. Gate observable: INITPAGES completes; $EFE4
          shows task 0 + enable bit set; serial banner still prints.
          Files to implement: mmu.rs (Mmu struct: 1024-byte map store,
          translate(logical_page, task)->physical_page, edit window
          $EFD0-$EFDF, control registers $EFE0-$EFE7, mmu_power_on_fill from
          config, task mask $FF->$3F, task-0 alias option); extend bus.rs
          (MMU translation path when enabled; io_rom_always config controls
          I/O/ROM decode precedence); tests/m2_mmu_gate.rs (INITPAGES pass;
          $EFE4 bit 7 set task 0; edit-window round-trip; task-1 $C000 ->
          physical $10000; banner persists; $FF->$EFE0 -> $EFE4 bits 5:0=$3F).
          Risk R-2 mitigated by mmu_power_on_fill knob.
        acceptance_criteria:
          - After BIOS INITPAGES read($EFE4) & 0xBF == 0x80 (enable bit set, task 0)
          - Write 16 known bytes to $EFD0-$EFDF task-0 edit window; read back identical
          - Task-1 logical $C000 translates to physical $10000 (not $0C000)
          - "Task-0 identity map: CPU $1234 reaches physical $01234"
          - Banner still printed after INITPAGES (no hang)
          - Write $FF to $EFE0; read($EFE4) & 0x3F == 0x3F
        dependencies:
          - m1

      - key: m3
        title: "WI-M3: XT-IDE controller, disk image, VIDEO ROM boot"
        type: task
        priority: P1
        description: >
          Implement the M3 milestone: XT-IDE ATA controller, raw disk image
          loader, and VIDEO ROM 60-sector boot path. Gate observable: 60 sector
          reads succeed without error; PC=$0800 logged. Files to implement:
          disk.rs (DiskImage: load raw flat binary via disk_image config key;
          read_sector; write_sector stub); xt_ide.rs (XtIde at $E300-$E30E:
          BSY/DRQ/ERR bits; READ SECTORS $20 state machine; SET FEATURES $EF;
          IDENTIFY $EC minimal 512-byte response; probe write tolerance
          $E300-$E330; trace logging every register access); extend bus.rs
          (XT-IDE decode; absent-device stubs: CH375 $E260-$E261, ESP
          $E100-$E102, Multi-I/O $E3FE-$E3FF returns open_bus / discards
          writes); tests/m3_xt_ide_gate.rs (VIDEO bank reset vector=$F000;
          probe writes no crash; 60 sector transfers; PC=$0800).
          Pre-implementation: parse rom.hex data records, log detected address
          ranges (Risk R-1). Risk R-3 mitigated by trace logging.
        acceptance_criteria:
          - "rom_bank=video: reset vector fetch returns $F000"
          - Probe writes $FF/$00 to $E300-$E330 do not crash; LBA 0 bytes unchanged
          - "SET FEATURES $EF: BSY clears; DRQ and ERR absent"
          - Sector-transfer counter reaches 60; no error-path output on stdout
          - CPU PC equals $0800 after 60th sector transfer (trace log or debug hook)
        dependencies:
          - m2

      - key: m4
        title: "WI-M4: DOS/65 cold boot, absent-device stubs"
        type: task
        priority: P1
        description: >
          Implement the M4 milestone: DOS/65 cold boot to A> prompt with all
          absent-device stubs. Gate observable: DOS/65 and A> on serial
          console. Files to implement: peripherals.rs (safe no-op stubs:
          CH375 $E260-$E261 reads->open_bus/writes discarded; Dual ESP
          $E100-$E102 same; Multi-I/O $E3FE-$E3FF keyboard self-test $AA->$55
          response, other reads->open_bus); extend bus.rs (wire peripherals.rs
          stubs; task-switching during loader for task-1 physical
          $10000-$11FFF; far-call stub at $FFF0 executes from ROM, no code
          synthesis needed per assumption A-6); tests/m4_dos_boot_gate.rs
          (physical $B800-$D870 non-zero after task-0 copy; physical
          $10000-$11FFF non-zero after task-1 copy; stdout contains DOS/65
          then A>; inject A:\r; echo received; no hang).
          Pre-implementation note (Risk R-4): disassemble ROM around $FFF0
          before coding to confirm far-call stub encoding matches BIOS source.
        acceptance_criteria:
          - Physical $B800-$D870 (task-0 map) is non-zero after loader task-0 copy
          - Physical $10000-$11FFF is non-zero after loader task-1 copy
          - stdout contains substring DOS/65 followed by A>
          - Inject A:\r; echo received; no timeout or fault
          - SIM device-init loop completes; no absent-device failure halts execution
        dependencies:
          - m3

      - key: m5
        title: "WI-M5: DOS/65 disk read/write, DIR listing"
        type: task
        priority: P1
        description: >
          Implement the M5 milestone: DOS/65 disk read/write and CP/M
          directory listing. Gate observable: DIR A: lists CP/M directory
          entries. Files to implement: extend xt_ide.rs (WRITE SECTORS $30:
          accept 512-byte write, commit to DiskImage; IDENTIFY $EC full
          512-byte response with fixed ASCII model string; bad-sector injection
          path: per-LBA configurable error return; B-drive LBA isolation
          $4100-$81FF disjoint from A $0000-$40FF); extend disk.rs
          (write_sector fully implemented; flush written sectors to raw image
          file); tests/m5_disk_io_gate.rs (from A>: inject DIR A:\r; assert
          CP/M directory entry; load .COM; assert output no hang; write small
          file read back byte-identical; drive E->failure no crash; bad-sector
          LBA->BAD SECTOR in stdout; Return->A> returns; B-drive write->A LBA
          0 unchanged).
        acceptance_criteria:
          - "DIR A: on the DOS prompt lists directory entries from the CP/M filesystem"
          - A valid .COM program loads and runs without hanging
          - Write then read-back of a small file produces byte-identical content
          - Accessing drive E returns failure; no crash or hang
          - Bad-sector injection causes BAD SECTOR in stdout; pressing Return continues to A>
          - B-drive write does not corrupt A-range (LBA 0-$40FF) sectors
        dependencies:
          - m4

      - key: m6
        title: "WI-M6: RTC model, config-file boot, configuration hardening"
        type: task
        priority: P1
        description: >
          Implement the M6 milestone: RTC model, full config-file boot, and
          configuration hardening. Gate observable: RTC read returns plausible
          date; emulator starts from config file with non-default settings.
          Files to implement: rtc.rs (Rtc struct: 16 nibble-only registers
          $EF90-$EF9F, low 4 bits only; control regs at offsets $0D-$0F;
          three clock policies host/fixed/epoch; STOP/RESET bit write sequence
          $02/$00/$00/$01/$05/$04 freezes then restarts); extend bus.rs (wire
          Rtc at $EF90-$EF9F; confirm open-bus for $EFA0-$EFCF, $EFF0-$EFFF,
          unassigned MMU offsets, physical holes above page $7F); extend
          config.rs (full TOML config-file loading via --config; open_bus
          documented with $EA as first-choice comment per Risk R-6);
          emulator/config/default.toml (acia_cts_default=true default per Risk
          R-5 mitigation); tests/m6_rtc_config_gate.rs (rtc_policy=host DOS
          time/date returns year 2020-2040; RTC write sequence no fault;
          rtc_policy=fixed known epoch date matches; CH375 C:->failure A>
          returns; Multi-I/O $AA->$55; read $EFA0->open_bus; --config
          custom.toml with rom_bank=video open_bus=0xEA rtc_policy=host boot
          succeeds).
        acceptance_criteria:
          - DOS/65 RTC read call returns plausible date matching configured RTC policy (year 2020-2040)
          - Firmware write sequence $02/$00/$00/$01/$05/$04 executes without fault; clock advances
          - rtc_policy=fixed with known epoch returns matching date
          - "CH375 absent: DOS C: access returns failure; no crash; A> returns"
          - Multi-I/O $AA command returns $55; init does not hang
          - Read $EFA0 equals open_bus config value
          - Emulator starts correctly from --config file specifying non-default rom_bank, open_bus=$EA, rtc_policy=host
        dependencies:
          - m5
```
