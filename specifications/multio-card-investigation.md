# PC6502 'Multio' (ISA Multi-I/O) Card — Investigation

**Investigation date:** 2026-07-03
**Scope:** Everything in this repository snapshot describing the card the human calls the "multio card," plus the current emulator implementation state, in preparation for emulating it.
**Target audience:** Emulator implementers (and the human requesting the work)
**Bead:** mc-22a

## 1. Result summary

- No file in this repository uses the literal word "multio." The human's term maps directly onto what the repo already calls the **ISA Multi-I/O card** (firmware constant `MULTIO_BASE`, `PC6502_firmware_source/bios_multi.asm:15`). See §2.
- Contrary to mc-22a's premise, this card is **extensively documented already** — in firmware source, in a dedicated per-domain investigation doc, and in the synthesized `system-reference.md`. It was omitted from mc-4c6/mc-hsy's close notes, not from the documents themselves. See §3.
- The card combines two independent functions on one I/O base: (1) an 8042/VT82C42-style PS/2 keyboard controller, and (2) a Centronics-style parallel-printer (LPT) port. Full register map and firmware driver behavior are documented in §3–§5.
- A partial emulator implementation exists (§6.1), but it has a **confirmed defect**: the boot-time keyboard self-test is wired to the wrong register address, so the real firmware's probe sequence would not actually succeed against today's emulator (§6.2). This is verified by direct inspection of the code and its test coverage, not speculation.
- The LPT/parallel-printer half of the card has **no emulator implementation at all**, not even a stub (§6.3).
- §7 gives the explicit list of what's missing to finish emulating this card, as requested by the acceptance criteria.

## 2. Search methodology and naming

Searched case-insensitively for `multio`, `multi-io`, `multi io`, and `multi I/O` across `specifications/`, `docs/`, `documentation/*.md`, `PC6502_firmware_source/`, `DOS65_OS/`, and `git log --all` (both `--grep` over commit messages and `-S` over source content). Also swept raw hex addresses (`E3E0`, `E3F0`–`E3FF`) in case any document referenced the card without using an I/O-name string.

No source in the repository uses the single word "multio." The repository's own name for this hardware is "ISA Multi-I/O card" / "Multi-I/O." This document treats "multio card" = "ISA Multi-I/O card" throughout, and does not use the word "multio" again below except in this note.

## 3. What's already documented

mc-22a's premise was that "multio" wasn't mentioned in the five source investigation documents' close notes. In fact, **all five** of the per-domain source documents that fed `system-reference.md` describe this card, in detail:

| Document | Relevant lines |
|---|---|
| `docs/investigation/io-registers.md` (per-domain source doc; not itself listed by name in system-reference.md's own citations, but is the fifth of the five per-domain investigations alongside the four below, and has the most detailed register table) | 24, 249, 290–304, 306–308 (video cross-reference), 319, 335, 364 |
| `specifications/hardware-spec.md` | 96, 140, 191, 207 (evidence entry `[E11]`) |
| `specifications/dos65-expectations.md` | 81, 211–214, 252, 270–271, 375 |
| `specifications/system-reference.md` (synthesized/authoritative merge of the above) | 75, 243, 384, 420–430, 449, 535, 548–551, 624, 704 |
| `specifications/memory-mmu.md`, `specifications/rom-reset.md` | No mentions (confirmed by grep) — expected, since MMU/reset are different domains and this device is a fixed I/O-window peripheral (§6.1). |

The rest of this section merges what these documents (and the firmware they cite) agree on, since none of them conflict with each other.

### 3.1 Register map

All three of `io-registers.md`, `hardware-spec.md`, and `system-reference.md` agree on this map. Nominal card base is `$E3E0` (`PC6502_firmware_source/bios_multi.asm:15`, `MULTIO_BASE = PC6502_IO+$3E0` where `PC6502_IO = $E000`, confirmed independently in `PC6502_firmware_source/testmulti.asm:6`) — but no firmware ever accesses the base address itself; only the offsets below are exercised (`docs/investigation/io-registers.md:292`).

| Address | Register | Direction | Notes |
|---|---|---|---|
| `$E3F0` | LPT data | Write | Printer data bits `PD7:PD0`. Initialized to `$00` (`bios_multi.asm:156-157`). |
| `$E3F1` | LPT status | Read | Bit 7 `/BUSY` (firmware tests only this bit; 1 = ready), bit 6 `/ACK`, bit 5 paper-out, bit 4 selected, bit 3 `/ERROR` (`bios_multi.asm:88-93`, `docs/investigation/io-registers.md:297`). |
| `$E3F2` | LPT control | Write | Bit 7 STAT1, bit 6 STAT0, bit 5 enable, bit 4 printer-interrupt control, bit 3 select, bit 2 reset, bit 1 line-feed, bit 0 strobe (`bios_multi.asm:95-100`). Firmware sequence: init writes `%00001000` then `%00001100`; each LPT byte send writes `%00001101` (strobe) then `%00001100` (`bios_multi.asm:158-162, 1082-1086`). |
| `$E3FE` | Keyboard data (`KBD_DAT`) | Read/write | Read: output-buffer/scancode byte. Write: keyboard-controller data byte, but only after status bit 1 reads 0 (`bios_multi.asm:16, 297-323`). |
| `$E3FF` | Keyboard status/command (`KBD_ST` / `KBD_CMD`, same address) | Read = status, write = command | Read bit 0: output data pending (1 = byte ready to read). Read bit 1: input-buffer busy (1 = don't write yet). Other status bits unused/unknown (`bios_multi.asm:17-18`, `docs/investigation/io-registers.md:300`, `system-reference.md:428`). |

This is a classic Intel-8042-style split (separate data port vs. combined status/command port), consistent with the firmware's own boot message identifying the expected part as a "VT82C42" (`bios_multi.asm:956`, message string `"KBD: VT82C42 NOT FOUND."`).

### 3.2 Keyboard behavior (`MULTIOINIT` → `KBD_PROBE`)

Firmware routine `KBD_PROBE` (`bios_multi.asm:171-234`) is the boot-time handshake:

1. Write command `$AA` (controller self-test) to `KBD_CMD` ($E3FF) via `KBD_PUTCMD` (`bios_multi.asm:173-174, 269-294`, which itself polls `KBD_ST` bit 1 for "not busy" before writing).
2. Read the response via `KBD_GETDATA` (`bios_multi.asm:176, 326-363`, which polls `KBD_ST` bit 0 for "data pending" before reading `KBD_DAT`); expect `$55`. Anything else prints `"KBD: VT82C42 NOT FOUND."` and the keyboard is treated as absent (`bios_multi.asm:179-190`).
3. On success: write command `$60` (set controller command register) then data `$20` ("translation disabled, mouse disabled, no interrupts") (`bios_multi.asm:218-223`).
4. `KBD_RESET` ($FF command, expects an ACK byte then swallows a follow-up response) (`bios_multi.asm:228, 382-401`).
5. `KBD_SETLEDS` ($ED command + a data byte encoding caps/num/scroll-lock state, each step expects ACK `$FA`) (`bios_multi.asm:230, 407-434`).
6. `KBD_SETRPT` ($F3 command + a typematic-rate byte, same ACK pattern) (`bios_multi.asm:231, 440-460`).

A full PS/2 scancode decoder (`KBD_DECODE`, `bios_multi.asm:517-902`) translates raw scancodes (including extended-prefix `$E0`/`$E1`, break codes `$F0`, modifier tracking, caps-lock case-swap, num-lock keypad remapping) into an internal keycode buffer consumed by `KBD_GETKEY`/`KBD_GETKEYB`/`KBD_GETSTATUS` (`bios_multi.asm:465-508`).

### 3.3 LPT behavior (`LPT_OUT`)

`LPT_OUT` (`bios_multi.asm:1063-1089`) polls `LPT_1` bit 7 (`/BUSY`) until ready (or times out, `LPT_WAITTO = $30FF`), writes the byte to `LPT_0`, then pulses `LPT_2` with strobe set then cleared. `LPT_OST` (`bios_multi.asm:1093-1096`) just returns that status bit. No interrupt-driven path, no bidirectional/status-register-read-back-of-data path.

## 4. Firmware source (`PC6502_firmware_source/`)

- **`bios_multi.asm`** (1096 lines) — the complete driver: register defines (§3.1), `MULTIOINIT`/`KBD_PROBE` (§3.2), full scancode decoder, `LPT_OUT`/`LPT_OST` (§3.3), and scancode-to-keycode mapping tables (`KBD_MAPSTD`/`KBD_MAPSHIFT`/`KBD_MAPEXT`/`KBD_MAPNUMPAD`, lines 975–1010). Two build variants are conditionally assembled via `.IFNDEF/.IFDEF PC6502BIOS` (lines 120–262): a verbose standalone-ROM variant with boot messages, and a silent variant for inclusion in the banked OS driver.
- **`testmulti.asm`** — a standalone diagnostic program, not previously cited in any specifications/ document. It independently redefines `PC6502_IO = $E000` (line 6, matching `bios_multi.asm`'s dependency), calls `MULTIOINIT` then loops on `KBD_GETKEY` echoing to a UART console (lines 21–42), and `.INCLUDE`s `bios_multi.asm` directly (line 232). The `Makefile` builds it as a real target: `testmulti.s19` (`Makefile:1,21-23,38-40`), separate from the main `dos65drv.s19`/`rom.hex`/`pcdos65.s19` build. This is a real, standalone hardware/emulator test vehicle for this card in isolation from the rest of DOS/65 — worth knowing about for emulator test design.
- **`dos65drv.asm`** — banked driver dispatch table: function 34 = `LPT_OUT`, function 35 = `KBD_GETKEY`, function 36 = `MULTIOINIT` (lines 83–85); functions 15–18 and 20–23 also point at `KBD_GETKEY`/`KBD_GETKEYB`/`KBD_GETSTATUS` (lines 60–67), i.e., the ESP-video and mapped-video console-selector groups both reuse this same keyboard driver for input. `.INCLUDE "bios_multi.asm"` at line 135.
- **`Makefile`** — line 66, the `pretty6502` formatting/lint step explicitly includes `bios_multi.asm` in its reformatting pass, alongside the other device drivers.

## 5. Boot integration (`DOS65_OS/`)

- **`DOS65_OS/dos65_os/simrbc.asm:118-120`** — the actual DOS/65 SIM cold-boot sequence:
  ```
  LDA     #36             ; MULTI IO INITIALIZE
  STA     farfunct
  JSR     DO_FARCALL
  ```
  This is called unconditionally during cold boot, alongside video/ESP/DSKY/RTC/IDE/CH375/floppy init calls (lines 91–120), and its return value is discarded exactly like the others (no branch on carry/A after the call). This is a more precise citation than what's in the existing specs, which describe this generically as "SIM cold boot ... Multi-I/O" (`dos65-expectations.md:81`) without pointing at the exact call site.
- Console-selector groups `$0E` and `$13` (`dos65-expectations.md:270-271`, `system-reference.md:548-551`) both depend on the Multi-I/O keyboard functions for input; `$13` is the VIDEO ROM's default/unattended group, so a working Multi-I/O keyboard is a prerequisite for a truly unattended VIDEO-bank boot (already flagged as such in `dos65-expectations.md:375` / `system-reference.md:551`).

## 6. Current emulator implementation (`emulator/src/*.rs`)

### 6.1 What exists

- **`emulator/src/bus.rs:25`** documents the intent: `$E3FE–$E3FF — Multi-I/O keyboard (absent device stub)`.
- **`emulator/src/bus.rs:107-108, 132`** wire CPU addresses `$E3FE-$E3FF` to `Peripherals::multiio_read`/`multiio_write`, computing `offset = addr - 0xE3FE` (so offset 0 = `$E3FE` data port, offset 1 = `$E3FF` status/command port).
- **`emulator/src/peripherals.rs:23-65`** — a `Peripherals` struct shared with the ESP and CH375 absent-device stubs, holding one bit of state: `kbd_selftest_pending: bool`.
- Architecturally: `Bus::read`/`Bus::write` (`bus.rs:57-83`) intercept the `$E000-$EFFF` I/O window directly on the CPU-visible address, before any MMU physical-address translation is applied (`translate_ram` is only reached in the RAM fallback arm). Any future Multi-I/O work — including finishing the register model or adding LPT — follows the same pattern: a new match arm in `io_read`/`io_write` keyed on the fixed logical address, no MMU interaction required. This is consistent with `specifications/memory-mmu.md:215` (§9.3, "I/O reads and writes are dispatched after physical translation" for the general RAM path; the I/O window itself bypasses that translation entirely by being matched first).

### 6.2 A confirmed defect: keyboard self-test is wired to the wrong port

The current implementation (`emulator/src/peripherals.rs:52-64`):

```rust
pub fn multiio_read(&mut self, offset: u8) -> u8 {
    if offset == 0 && self.kbd_selftest_pending {
        self.kbd_selftest_pending = false;
        return 0x55;
    }
    self.open_bus
}

pub fn multiio_write(&mut self, offset: u8, val: u8) {
    if offset == 0 && val == 0xAA {
        self.kbd_selftest_pending = true;
    }
}
```

Both branches check `offset == 0`, i.e., address `$E3FE` — the **data** port. But per §3.2 and `bios_multi.asm:173-174`, real firmware writes the `$AA` self-test command to `KBD_CMD` at `$E3FF` (offset 1, the **status/command** port), via `KBD_PUTCMD`'s `STA KBD_CMD` (`bios_multi.asm:292`). It only ever reads the response from `$E3FE` (which the emulator does model correctly on the read side, address-wise).

Concretely: when real firmware executes `LDA #$AA / JSR KBD_PUTCMD` (`bios_multi.asm:173-174`), the emulator sees a write to offset 1 (`val=0xAA`) that `multiio_write` ignores entirely (it only checks `offset == 0`), so `kbd_selftest_pending` is never set. From here, since `$E3FF`'s status bits are not modeled at all (that address always returns raw `open_bus`, regardless of `kbd_selftest_pending`), the exact failure path `KBD_PROBE` takes depends purely on the configured `open_bus` byte's bit pattern (`bios_multi.asm:171-206`):
  - If `open_bus` bit 1 (IBF) reads as 1: `KBD_PUTCMD`'s own polling loop (`bios_multi.asm:275-289`) never sees "not busy," times out without ever issuing the write, and `KBD_PROBE`'s `BCS KBD_TIMEOUT1` fires → `"KBD: VT82C42 WRITE TIMEOUT."`.
  - Else if `open_bus` bit 0 (OBF) reads as 0: the write is issued (and harmlessly discarded, since offset 1 has no handler), but `KBD_GETDATA`'s polling loop (`bios_multi.asm:336-354`) never sees "data pending," times out without ever reading `$E3FE`, and `BCS KBD_TIMEOUT2` fires → `"KBD: VT82C42 READ TIMEOUT."`.
  - Else (`open_bus` bit 0 = 1 by coincidence): `KBD_GETDATA` proceeds to read `$E3FE` (offset 0, `bios_multi.asm:361`) immediately, gets `self.open_bus` (not `$55`, since `kbd_selftest_pending` was never set), and the `CMP #$55` check fails → `"KBD: VT82C42 NOT FOUND."` (`bios_multi.asm:179-190`).

  All three outcomes are dead ends for keyboard detection; which one fires is an accident of the configured `open_bus` byte, not deliberate emulation of the handshake. Regardless of which path is taken, the failure does not propagate: `MULTIOINIT` calls `KBD_PROBE` without ever checking its returned carry flag (`bios_multi.asm:146`), falls straight through into LPT init, and returns `CLC` (success) unconditionally (`bios_multi.asm:164-165`) — consistent with SIM cold boot ignoring all device-init return values anyway (§5). So this defect would surface as a boot-time error message and a nonfunctional keyboard, not a hang or a failed boot.

This is not a hypothetical bug — it's confirmed by the implementation's own test suite, which encodes the same (address-wise incorrect, relative to real firmware) assumption rather than validating against real firmware:

- `emulator/tests/m4_dos_boot_gate.rs:170-173`: `bus.write(0xE3FE, 0xAA); // self-test command on offset 0` then asserts `bus.read(0xE3FE) == 0x55`. This is a direct, synthetic bus-level poke — not driven by CPU execution of real firmware. (The same test file's "full emulation" section builds a hand-written synthetic boot ROM via `build_dos65_boot_rom()`, `m4_dos_boot_gate.rs:60+` — it does not assemble or run the real `bios_multi.asm`/`6502PCbios.asm`, so this gap is not exercised end-to-end anywhere in the gate-test suite.)
- `emulator/tests/m6_rtc_config_gate.rs:84-90` (`multiio_selftest_aa_55`, labeled `REQ-M6 item 4 / BR-7`) does the identical `bus.write(0xE3FE, 0xAA)` / `bus.read(0xE3FE) == 0x55` check.

This traces back to when `peripherals.rs` was first scaffolded — the `offset == 0` logic for both read and write has been present, unchanged, since the very first commit that created the file (`0782bcb` / `8348ec3`, "scaffold Cargo workspace for PC6502 emulator," 2026-06-27/28) and was never revisited. Notably, commit `f8ffd6c` ("implement deferred gate tests with real disk image," 2026-07-01) specifically reverse-engineered the ESP and CH375 stub *return values* against the real DOS/65 driver's exact polling code (citing specific driver addresses like `$C948`/`$C95C` in the commit message) — but did not revisit Multi-I/O's port addressing or status-bit semantics in the same pass.

At the requirements level, `plans/pc6502-emulator-milestones/requirements.md:238` (REQ-M6 item 4) states the contract in address-agnostic terms — "injecting `$55` response to `$AA` command passes controller init" — and the coverage matrix marks `REQ-M6 | covered` (`requirements.md:251`) and `BR-7 | covered`. `system-reference.md:704`'s implementation-status checklist likewise marks this `[x]` done, citing `peripherals.rs:52-64`. Both of those "covered"/`[x]` determinations rest entirely on the synthetic `$E3FE`-only unit tests described above, not on any test that drives the real firmware's actual `$E3FF`-targeting code path.

### 6.3 LPT: no implementation at all

`$E3F0-$E3F2` (LPT data/status/control, §3.1/§3.3) do not appear anywhere in `bus.rs`'s `io_read`/`io_write` match arms. They fall through to the generic catch-all (`bus.rs:118` `_ => self.open_bus` for reads, `bus.rs:142` `_ => {}` discard for writes) — the same handling any random unmapped `$E000-$EFFF` address gets. This is an address-decode gap, not a deliberate "absent device" stub the way the keyboard half received; nothing in the codebase treats LPT as a named device at all.

`plans/pc6502-emulator-milestones/requirements.md:253-262` ("Out Of Scope") does explicitly scope Multi-I/O, but only the keyboard side — line 262: *"Multi-I/O beyond keyboard self-test (`$AA`/`$55`): mouse port, IRQ routing, and remaining status bits are out of scope."* This confirms the mouse port, IRQ routing, and remaining status bits (§7 item 6) were deliberately deferred, not merely overlooked. LPT specifically is not named anywhere in this bullet or elsewhere in the Out Of Scope list — it is absent by omission (never in scope, never explicitly excluded), unlike "Video card" or "Floppy drive," which are each called out by name.

### 6.4 Git history summary

- `d65a027` ("work," 2026-06-29) — the bulk vendor-import commit that added the `DOS65_OS/` and `PC6502_firmware_source/` trees wholesale (including `bios_multi.asm`, `testmulti.asm`, `simrbc.asm`); it matched grep hits for "MULTIO" only because those files arrived in this one large drop, not because of any multio-specific change.
- `0782bcb` / `8348ec3` (2026-06-27/28, Cargo workspace scaffold) — `peripherals.rs` created with today's `offset == 0` logic already in place (§6.2).
- `f8ffd6c` (2026-07-01, "implement deferred gate tests with real disk image") — tuned ESP/CH375 stub values against real driver behavior; did not touch Multi-I/O logic.
- `3496ac0` (2026-06-28, "implement WI-M4 — DOS/65 cold boot, task-switching, absent-device stubs") — added the M4 gate test exercising Multi-I/O at `$E3FE` only.
- `5b67515` (2026-06-29, "implement RTC-72421 model, config hardening, and M6 gate tests," on `main`) — added `multiio_selftest_aa_55` (§6.2), same `$E3FE`-only assumption.

## 7. What's missing to actually implement/finish emulating this card

1. **Fix the self-test port mismatch** (§6.2): real firmware writes `$AA` to `$E3FF` (offset 1), not `$E3FE` (offset 0). Whether to fix the emulator's offset check or explicitly decide the stub's current behavior is intentional (and if so, document why) is an implementation/product decision for whoever picks this up — this document only establishes that the two disagree.
2. **Model the status register (`$E3FF`) for real.** Bits 0 (output-data-pending) and 1 (input-buffer-busy) currently always return raw `open_bus`, never reflecting `kbd_selftest_pending` or any other internal state. Firmware's `KBD_PUTCMD`/`KBD_PUTDATA`/`KBD_GETDATA` polling loops (§3.2) are not genuinely modeled today — they merely happen to proceed or stall depending on the configured `open_bus` byte's bit pattern.
3. **Model the post-self-test configuration handshake:** command `$60`/data `$20` (disable translate/mouse/interrupts), `KBD_RESET` (`$FF`), `KBD_SETLEDS` (`$ED` + state byte), `KBD_SETRPT` (`$F3` + rate byte) — each expects a `$FA` ACK response. None of this is modeled; every one of these writes is presently silently discarded with no ACK ever returned. Firmware's retry loops are finite (`KBD_WAITTO = $30FF`), so this becomes a boot-time delay/degraded-keyboard condition rather than an infinite hang, but this has never actually been confirmed against real firmware execution either way (§6.2's point about the M4 test's synthetic boot ROM not exercising real `bios_multi.asm`).
4. **No scancode injection path exists.** Nothing feeds real or synthetic PS/2 scancodes into `KBD_DAT` for `KBD_DECODE`/`KBD_GETKEY` to consume. The boot-time self-test handshake is a distinct problem from actual interactive keyboard input (e.g., from a host keyboard) — the latter has zero implementation today, stub or otherwise.
5. **LPT (`$E3F0-$E3F2`) has no code at all** (§6.3) — no register state, no `/BUSY` modeling, no captured output. If the goal includes printer emulation (e.g., capturing print output to a file), this is a green-field addition, not a fix.
6. **Hardware unknowns no local source resolves** (carried forward from `system-reference.md`'s R2.6 and `io-registers.md`'s open questions, reconfirmed still open by this investigation). Note the first two sub-items were explicitly deferred by `requirements.md:262` ("out of scope"), not merely undiscovered — the milestone plan made a deliberate call not to chase them, rather than missing them:
   - Whether the 8042-style controller has a mouse port, and if so its protocol/address — firmware only disables it (`$20` command byte, bit for "mouse disabled"). Explicitly out of scope per `requirements.md:262`.
   - IRQ routing/jumper wiring for keyboard or LPT interrupts. The LPT control register (`$E3F2`) has a "printer-interrupt control" bit (§3.1), but its electrical effect is undocumented anywhere in this repo. Keyboard IRQ routing is explicitly out of scope per `requirements.md:262`; LPT IRQ routing is simply never mentioned (same omission as the rest of LPT, §6.3).
   - Full meaning of status-register bits beyond bits 0/1 of `$E3FF`, and bits 2:0/5:6 of `$E3F1`.
   - VT82C42 vs. generic-8042 command-set differences (firmware only exercises a small command subset: `$AA`, `$60`+`$20`, `$FF`, `$ED`, `$F3`).
   - No schematic, datasheet, PCB photo, or programmable-logic source for this specific expansion card exists anywhere in `documentation/` — only the general board document (`documentation/PC6502_system_documentation.md`, cited as `[E1]`/`[L1]` elsewhere), which lists Multi-I/O as one of several slot-fitted optional cards, not a baseboard-guaranteed device.
   - No confirmed mechanism (jumper/DIP switch) for how a card's base address would be configured/relocated in a physical slot — `$E3E0` is only what this firmware assumes, matching `io-registers.md`'s general open question #7 ("How are card address bases configured...").

If the goal is full interactive emulation (real keyboard input and printer capture, not just passing the firmware boot probe), items 1–5 are implementation work answerable from this repository alone. Items in bullet 6 need either a datasheet/schematic for the physical card or hardware measurement of a real unit — this repository cannot resolve them on its own.

## 8. Evidence index

- **[F1]** `PC6502_firmware_source/bios_multi.asm:1-1096` — full Multi-I/O driver: register defines, `MULTIOINIT`/`KBD_PROBE`, scancode decoder, `LPT_OUT`/`LPT_OST`, mapping tables, boot messages.
- **[F2]** `PC6502_firmware_source/testmulti.asm:1-233` — standalone Multi-I/O diagnostic program and build target, not previously cited in specifications/.
- **[F3]** `PC6502_firmware_source/dos65drv.asm:60-67, 83-85, 135` — banked dispatcher functions 15-18/20-23/34-36 and `bios_multi.asm` inclusion.
- **[F4]** `PC6502_firmware_source/Makefile:1, 13, 21-23, 38-40, 66` — `testmulti.s19` build target and `bios_multi.asm` build/lint integration.
- **[F5]** `DOS65_OS/dos65_os/simrbc.asm:118-120` — SIM cold-boot call to farfunct 36 (`MULTIOINIT`).
- **[F6]** `emulator/src/bus.rs:12-30, 57-145` — I/O address decode, including Multi-I/O match arms and the absence of any LPT match arm.
- **[F7]** `emulator/src/peripherals.rs:1-65` — `Peripherals` struct and `multiio_read`/`multiio_write` implementation (the offset-0/offset-1 discrepancy, §6.2).
- **[F8]** `emulator/tests/m4_dos_boot_gate.rs:1-11, 60+, 150-173` — M4 gate test; synthetic boot ROM; Multi-I/O bus-level assertions at `$E3FE` only.
- **[F9]** `emulator/tests/m6_rtc_config_gate.rs:84-90` — `multiio_selftest_aa_55` unit test.
- **[F10]** `plans/pc6502-emulator-milestones/requirements.md:108-118 (BR-7), 233-251 (REQ-M6), 253-262 (Out Of Scope)` — formal requirement text and scope boundary.
- **[F11]** Git commits: `d65a027` (bulk vendor import), `0782bcb`/`8348ec3` (Cargo scaffold, origin of the offset-0 logic), `f8ffd6c` (ESP/CH375 tuning that didn't extend to Multi-I/O), `3496ac0` (M4 gate test), `5b67515` (M6 gate test, on `main`).
- **[F12]** `docs/investigation/io-registers.md:24, 249, 290-308, 319, 335, 338-364` — original per-domain register investigation (most detailed independent source; predates and feeds `system-reference.md`).
- **[F13]** `specifications/hardware-spec.md:96, 140, 191, 207` — evidence entry `[E11]` and optional-device framing.
- **[F14]** `specifications/dos65-expectations.md:81, 195-220, 245-275, 365-380` — boot-sequence dispatcher table and console-selector integration.
- **[F15]** `specifications/system-reference.md:70-80, 238-248, 378-390, 410-430, 440-455, 525-555, 610-630, 695-710` — synthesized decode table, boot sequence, register table, R2.6 open question, implementation-status checklist.
- **[F16]** `specifications/memory-mmu.md:205-215` — general I/O-window-vs-MMU-translation architecture (§9.3), corroborating that I/O devices including this one sit outside MMU translation.

---

*Scope note: this investigation covers only what the bead asked — documentation state and emulator-implementation state for the ISA Multi-I/O card. While tracing git history and the milestone plans, I noticed the M4/M6 gate-test suite's broader pattern of validating several other "absent-device stub" claims (ESP, CH375) against real driver polling code, but only Multi-I/O against a synthetic assumption of its own making — that pattern may be worth a wider look if the human wants confidence in the other stubs too, but that's outside this bead's scope and is mentioned here only per the "mention unrelated findings, don't act on them" instruction.*
