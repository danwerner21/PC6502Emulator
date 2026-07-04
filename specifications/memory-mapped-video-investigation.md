# PC6502 Memory-Mapped Video Card — Emulation Requirements and Display Frontend Investigation

**Investigation date:** 2026-07-04
**Scope:** What the memory-mapped video card (physical pages `$F8` and up, distinct from the M3 VIDEO-ROM boot path) needs to emulate to support real graphical/text programs, and how to surface its output to a human. Investigation only — no emulator code written.
**Target audience:** Emulator implementers, and whoever picks up the follow-on milestone(s)
**Bead:** mc-ehx

## 1. Result summary

- The card is already documented at the register/memory-map level by two independent firmware sources that agree on the shared subset: the OS driver (`PC6502_firmware_source/bios_video.asm`) and DBASIC's graphics build (`DOS65_OS/software/dbasic/screencmds.asm`). Prior investigations (`hardware-spec.md`, `memory-mmu.md`, `system-reference.md`) already captured the register list and the "32 KiB claim vs. offsets beyond it" conflict (C8/R2.3). This document adds the DBASIC evidence (not previously cited for video), reconciles both register-map comments into one table, and derives the arithmetic that explains *why* the 32 KiB claim and the `$BFFF`-offset claim conflict (§3.2) — not just that they do.
- Of the four named programs, **only DBASIC's memory-mapped-screen build (`DBASICMP.COM`) exercises true bitmap graphics** (LORES/HIRES/DOUBLE/QUAD/MONO, line drawing, custom character patterns, direct peek/poke). DOS/65 itself, Speedscript, and wyrmhold are all **text-mode only** — wyrmhold's "graphics" are custom 8×8 character-generator glyphs composited in text cells, not a bitmap mode. See §4.
- The current emulator (`emulator/src/`) implements **nothing** for this card beyond the unrelated M3 ROM-bank boot path. Physical pages `$F8` and up are not backed by RAM; MMU-mapping them (as real firmware does) currently reads open-bus and discards writes. There is no video-related config surface at all — not even a dead placeholder field. See §5.
- Two independent, real programs (the OS driver and Speedscript) map the video pages into **two different logical CPU windows** (`$B000` vs. `$A000`) depending on whether the caller is the OS driver itself or an application doing direct VRAM access. An emulator's video model must not assume a fixed logical address; only the *physical* pages matter. See §3.3.
- **Recommendation: a local web server streaming the framebuffer to a browser canvas is the right primary frontend**, not mainly because of graphics fidelity (a native window would do as well) but because this project's actual runtime environment is a headless-by-default agent/CI sandbox with no guaranteed display server — a browser reachable over a port sidesteps that entirely. A `minifb`-based native-window feature is worth adding later for a developer running the emulator on their own desktop, reusing the same framebuffer model. See §6.
- A three-milestone breakdown (proposed `M7`–`M9`, continuing the existing M1–M6 numbering) is given in §7, in the same style as `plans/pc6502-emulator-milestones/decomposition.md`. No beads have been created for these; that is left to whoever picks this up.

## 2. Method and how this builds on prior investigations

Searched `specifications/hardware-spec.md`, `specifications/system-reference.md`, `specifications/memory-mmu.md`, `docs/investigation/io-registers.md`, `plans/pc6502-emulator-milestones/requirements.md`, and `specifications/multio-card-investigation.md` (the explicitly-flagged prior-lesson document) before touching firmware source, per this bead's instruction not to conclude "undocumented" from a keyword search alone. Unlike the Multi-I/O card (which turned out to be fully documented under a name mismatch), the video card's documentation gap here is real but narrow: the *register/memory-map facts* were already captured (mostly from `bios_video.asm` alone); what was missing was (a) the second, richer register-map source in DBASIC, (b) any analysis of what application software actually does with the hardware, and (c) any emulator-implementation or frontend analysis at all. This document is additive to, not a correction of, `hardware-spec.md` §6.2/§9, `memory-mmu.md` §2.3/§10.3, and `system-reference.md` §8 (C8)/§9 (R2.3) — it cites and builds on all three rather than re-deriving them.

`docs/investigation/io-registers.md:306-308,366` explicitly declines to duplicate video-card detail ("Its register/memory contract belongs with the video and MMU investigations and is intentionally not duplicated here") — this document is that dedicated investigation for the display/graphics side (as distinct from the M3 VIDEO-ROM boot path, which is a different concern — see §3.4).

## 3. Hardware: what's documented and how it reconciles

### 3.1 Register map (reconciled from two independent sources)

Two firmware files carry near-identical header comments describing the register layout, differing in one detail. Both are real, both are cited elsewhere (`hardware-spec.md [E12]`, `memory-mmu.md [E9]`), but only `bios_video.asm`'s version had been quoted in prior docs.

| Offset (within mapped page) | Meaning | `bios_video.asm:24-41` | `screencmds.asm:6-23` |
|---|---|---|---|
| `$00` | Scanline emulation: `$01`=soft on, `$02`=off | Yes | Yes |
| `$01` | Display page: `$01`=page1, `$02`=page2 | Yes | Yes |
| `$02` | Character-generator write offset (`data<<3`) | Yes | Yes |
| `$03` | Character-generator write data (auto-advances 8 rows) | Yes | Yes |
| `$04` | EXECUTE command: `$00` reset-default, `$01` reset-saved, `$02` save-current | Yes | Yes |
| `$05` | Text mode: `$01`=on, `$02`=off | Yes | Yes |
| `$06` | Lores mode: `$01`=on, `$02`=off | Yes | Yes |
| `$07` | Double lores: `$01`=on, `$02`=off | Yes | Yes |
| `$08` | Hires mode: `$01`=on, `$02`=off | Yes | Yes |
| `$09` | Double hires: `$01`=on, `$02`=off | Yes | Yes |
| `$0A` | 80-column: `$01`=on, `$02`=off | Yes | Yes |
| `$0B` | Mixed mode: `$01`=on, `$02`=off | Yes | Yes |
| `$0C` | Quad hires: `$01`=on, `$02`=off | Yes | Yes |
| `$0D` | Mono hires: `$01`=on, `$02`=off | Yes | Yes |
| `$0E` | Multicolor: `$01`=on, `$02`=off | **Not listed** | Yes (`VideoMulticolor = $A00E`) |

`screencmds.asm:5` additionally states outright what `bios_video.asm` leaves implicit: "VIDEO CARD IS A 32K AREA (**MAPPED IN BANKS TO $AXXX**)" — i.e. this source is explicit that the register/VRAM offsets are relative to whichever logical page the caller chooses to map the bank into, here `$A000-$AFFF`, vs. `bios_video.asm`'s own convention of `$B000-$BFFF` (§3.3).

Register `$0E`/`VideoMulticolor` is set by DBASIC's text-mode setup (`screencmds.asm:371-374,378-380`, a "40/80 column multicolor" text variant) but **no code anywhere in the reviewed sources ever reads it back or branches on it in a plot routine**, and the "Multicolor Page 1/2" VRAM ranges it implies (`screencmds.asm:30-31`, `$3000-$377F`/`$3800-$3F7F`) are never addressed by any arithmetic in `screencmds.asm`'s `V_PLOT`/`V_SPEEK`/`V_SPOKE`. Treat multicolor mode as **documented but functionally unexercised** by any reviewed software — lower confidence than every other register in the table, and not worth emulating precisely until something is found that actually uses it.

### 3.2 VRAM layout, bank arithmetic, and why the 32 KiB conflict exists

`hardware-spec.md:99,156` and `memory-mmu.md:48-50` already flag that the driver calls the card "32 KiB" while documented mode offsets run through `$BFFF`, and mark this an unresolved conflict (C8/R2.3). Both `bios_video.asm:43-51` and `screencmds.asm:25-34` list the same offset ranges:

```
$1000-$177F   40/80 Text Page 1        $3000-$377F   Color Multicolor Page 1 (screencmds.asm only)
$1800-$1F7F   40/80 Color Page 1       $3800-$3F7F   Color Multicolor Page 2 (screencmds.asm only)
$2000-$277F   40/80 Text Page 2
$2800-$2F7F   40/80 Color Page 2
$2000-$5FFF   HIRES PAGE 1
$6000-$8FFF   HIRES PAGE 2
$2000-$BFFF   DOUBLE HIRES
```

These are **not** CPU or physical addresses. Reconciling them against the actual bank-selection arithmetic in `screencmds.asm`'s `V_SPEEK`/`V_SPOKE` (`screencmds.asm:199-227,231-268`) and `V_PLOT_HIRES_COLOR`/`V_PLOT_HIRES_MONO` (`screencmds.asm:773-784,890-901`) shows what they actually are: a **linear "virtual VRAM address"** whose top nibble selects a physical bank (`bank = VIDEOBANK + nibble`, masked `AND #$07` in the SPEEK/SPOKE path) and whose low 12 bits are the offset within that bank once mapped to logical `$A000-$AFFF`. Under that reading:

- Nibble `$0` (bank `$F8`) holds registers + character generator only — correctly absent from the "VRAM" table, since it isn't bulk memory.
- Nibble `$1` (bank `$F9`) is Text Page 1 (offset `$000-$77F`) + Color Page 1 (offset `$800-$F7F`) — this is exactly `bios_video.asm`'s own `CLEARSCREEN` (`bios_video.asm:166-195`) and `video.asm`'s direct-VRAM layout (§4.4), just renumbered with a `$1xxx` prefix instead of a bank-relative offset. Confirmed, load-bearing.
- Nibble `$2` (bank `$FA`), "Text/Color Page 2": **no code reviewed anywhere (OS driver, Speedscript, DBASIC, wyrmhold) ever maps or writes this bank as a second text page.** Register `$01` ("page1/page2") is defined but never set to `$02` by any reviewed source. Documented, never exercised.
- "HIRES PAGE 1" (nibbles `$2`-`$5`, banks `$FA`-`$FD`) and "HIRES PAGE 2" (nibbles `$6`-`$8`, banks `$FE`-`$100`) together match the starting bank (`VIDEOBANK+2` = `$FA`) that `V_PLOT_HIRES_COLOR`/`V_PLOT_HIRES_MONO` actually compute from a pixel address (`screencmds.asm:773-784`) — **but bank arithmetic is an unsigned 8-bit page number**, and nibble `$8` alone already computes `$F8+8 = $100`, which overflows. "DOUBLE HIRES" (nibbles `$2`-`$B`) reaches nibble `$B` → bank `$103`.
- **This is the mechanism behind the previously-flagged C8/R2.3 conflict**: the documented offset ranges assume more distinct 4 KiB banks than an 8-bit physical page number can address starting from `$F8` (only 8 banks fit: `$F8`-`$FF`, nibbles `$0`-`$7`). Above that, the source's own address-to-bank arithmetic silently wraps or is undefined — this is a latent defect in the **original hardware/firmware design as documented**, not something an emulator should try to faithfully reproduce without further hardware guidance. (This bank/nibble derivation is my own reconciliation of the two comment blocks against the working plot/peek code, not itself independently confirmed against real hardware — flagged as **inferred**, consistent with this project's existing hedging convention.)
- **Practical conclusion for emulation:** back physical pages `$F8`-`$FF` (8 banks, 32 KiB total) — this is the largest range any reviewed code path can address without wraparound (`AND #$07` in `V_SPEEK`/`V_SPOKE` cleanly caps at `+7`), covers every register, both proven text/color pages, and single/double/quad/mono HIRES up to the point where the source's own arithmetic stays in range. Treat "Text/Color Page 2" and the un-addressable tail of "DOUBLE HIRES" as unimplemented/out of scope, matching `memory-mmu.md:50`'s existing recommendation not to allocate `$F8-$FF` "without a separate video-card specification" — this document is that specification.

### 3.3 Two different logical-page conventions in real code

`bios_video.asm` always maps the video bank into **logical page `$B`** (`LDX #$0B` at `bios_video.asm:60,142,167`; register self-test/mode-set at `$B00x`, char/color RAM at `$B000-$BFFF`). Every application-level direct-VRAM routine reviewed — Speedscript (`defines.asm:19-23`), DBASIC's graphics build (`screencmds.asm:217-219,258-260,577-579`), and wyrmhold (`video.asm:19-26`, `tiles.asm:25-32`) — instead maps the same physical pages into **logical page `$A`** (`LDX #$0A`). wyrmhold's `video.asm:1-13` header states the reason directly: the OS driver's FARCALL-based text output "does NOT page; it uses FARCALL chrout/locate and must be used while in task 0," while the direct-VRAM "fast path" needs its own explicit page mapping and deliberately picked a different logical window than the driver's own `$B000` so the two don't collide if mixed. **An emulator's video model must therefore be addressed by physical page (`$F8`+bank), never by a fixed logical/CPU address** — the MMU's existing generic page-translation (`emulator/src/mmu.rs:63-78`) already provides this; the video subsystem just needs to be a real memory-like target reachable through it, not a shortcut keyed to `$A000` or `$B000`.

### 3.4 Boot/dispatch integration and how this differs from the M3 scope

The banked OS driver's function-dispatch table (`PC6502_firmware_source/dos65drv.asm:41-123`) carries the video driver's entry points across two groups:

| Functions | Purpose | Used by |
|---|---|---|
| 19-23 | `WRVID`/keyboard/`VIDEOINIT` — the primary group wired to console selector `$13` | DOS/65 boot when `CONSOLE=$13` (`system-reference.md:535,549`) |
| 37-39, 56-59 | `SETXY`/`CLEARSCREEN`/`SETCOLOR`/`SCROLLUP`/`SETMODE`/`FPAINTCURSOR`/`UNPAINTCURSOR` — an extended API for direct cursor/screen control | Called explicitly via `farfunct`+`JSR $FFF0` by DBASIC's memory-mapped build (`dbasic.asm:60-69,588-600`, `screencmds.asm:112-121,141-144,185-188`) |

This is entirely separate from the M3 milestone's "VIDEO ROM" concern (bead `pe-1rx`): that milestone is about the **boot-time ROM bank** named `VIDEO` (reset vector behavior, 60-sector XT-IDE auto-boot, console selector defaulting to `$13` — `system-reference.md §5.5`, `rom-reset.md §6.3`). Once DOS/65 is running, whether `CONSOLE=$13` was set by the VIDEO ROM bank or the human runs `WYRMHOLD`/`DBASICMP` from the Base-bank Supermon prompt over a already-established serial link makes no difference to the video-card driver itself — this document's scope is the card's own register/memory contract and the software built on it, independent of which boot path got the OS running.

## 4. What each named program actually does with the card

### 4.1 DOS/65 itself (the OS's own driver, `bios_video.asm`)

`bios_video.asm` is the OS driver — compiled into the banked `$C000` driver (`hardware-spec.md:131`) and wired to console selector `$13` (§3.4). At `VIDEOINIT` (`bios_video.asm:58-115`) it: maps physical `$F8` into logical `$B` (task 1), runs a presence self-test (write `$00` then `$FF` to offset `$06`, expect readback to match — `bios_video.asm:64-73`), explicitly clears every graphics-mode register to off (`$02`) — lores, double-lores, hires, double-hires, mixed, quad-hires, mono (`bios_video.asm:75-82`) — then sets text mode, page 1, and 80-column on (`bios_video.asm:84-89`). **DOS/65's own driver never enables a graphics-mode register anywhere in this file.** Character output (`WRVID`, `bios_video.asm:238-320,339-386`) handles CR/LF/backspace and scrolls at row 24 by copying bytes within the mapped char/color window (`SCROLLUP`, `bios_video.asm:442-526`) — no hardware scroll-offset register is used. The cursor is a software block cursor implemented by swapping the color byte under it (`PAINTCURSOR`/`UNPAINTCURSOR`, `bios_video.asm:390-432`). **Conclusion: text mode only, 40 or 80 columns, needs only physical pages `$F8` (registers, self-test) and `$F9` (char/color RAM).**

### 4.2 Speedscript (`DOS65_OS/software/speedscript/`)

A word processor. `defines.asm:19-23` defines `SETPAGE`, `VIDEOBANK=$F8`, and `VIDTEXT_PAGE=$01+VIDEOBANK` (i.e. `$F9`) directly rather than going through the OS driver's dispatch table. `screen.asm:331` calls `SETPAGE` directly ("Y = video sub-page (set by caller)"). `io.asm:9-32` (`INIT`) sets 80-column text mode and clears the screen via `farfunct` calls (`FC_SETMODE`, `FC_COLOR`, `FC_SCNCLR` — the OS driver's extended-API group, §3.4) — same driver-mediated setup DOS/65 itself uses. For bulk screen updates it then bypasses the driver and writes VRAM directly for speed: `io.asm:477-479` sets a raw pointer to `$A050` ("video pointer... row 1"), i.e. logical page `$A`, and `io.asm:513-516` pokes characters straight into it through `(indir),Y`. **Conclusion: text mode only (80 columns), same two physical pages as DOS/65, but exercises the direct-VRAM/page-`$A` path (§3.3) as well as the driver path — an emulator needs both to be correct, not just the driver-mediated one.**

### 4.3 DBASIC (`DOS65_OS/software/dbasic/`) — two distinct build variants

The Makefile (`Makefile:2,4-14`) builds two different `.com` files from the same `dbasic.asm`, selected by the `-D MEMORYMAPPEDSCREEN` assembler flag:

- **Plain `dbasic.com`:** `iovect.asm:7-40` shows character I/O goes through `PEM` calls (DOS/65's own CP/M-style entry points — function `2`=console out, `11`=console status), i.e. whatever DOS/65's boot-time `CONSOLE` selector already points at. `dbasic.asm` and `iovect.asm` contain **zero** direct references to video hardware (`$B0`/`$A0`/`$F8`/`$F9`/`SETPAGE`/`VIDEOBANK` all return no matches) — confirmed by direct grep, not inferred. Whatever text this build shows on a video console, it shows purely because DOS/65's own driver is doing the work underneath it; DBASIC itself needs nothing from the card. `screencmds.asm:1232-1248` (the `.ELSE` branch of the same file) makes this a *language-level* fact too: when `MEMORYMAPPEDSCREEN` isn't defined, `SPEEK`/`SPOKE`/`SCRCLR`/`SCREEN`/`PATTERN`/`PLOT`/`LOCATE`/`COLOR`/`LINE` all compile to hard syntax errors — the plain build can't even parse a program that tries to use graphics.
- **Memory-mapped `dbasicmap.com`** (shipped on the stock disk image as `DBASICMP.COM` — `system-reference.md`'s disk-content finding lists it among the 24 `.COM` files present): this is by far the richest consumer of the card in the repository. `screencmds.asm` implements a full BASIC graphics API: `SCREEN` mode select — text (40/80/multicolor), LORES (single/double, mixed), HIRES (single/double/quad/mono, mixed) (`screencmds.asm:270-566`); `PLOT x,y,color` with mode-specific pixel packing — 2 vertically-packed nibble pixels/byte in LORES (`screencmds.asm:625-709`), 2 horizontally-packed nibble pixels/byte in HIRES color (`screencmds.asm:711-820`), 8 monochrome bits/byte in HIRES mono via bit-lookup tables (`screencmds.asm:822-931`); `LINE x1,y1,x2,y2,color` — a software Bresenham line drawer built entirely on `PLOT` (`screencmds.asm:966-1231`, no hardware line-draw); `PATTERN n,...` — user-definable character-generator glyphs via the `$02`/`$03` registers (`screencmds.asm:941-964`); and `SPEEK`/`SPOKE` — raw peek/poke into any of the 8 video banks (`screencmds.asm:199-268`, the source of the bank-arithmetic derivation in §3.2). Cold-start also uses the OS driver's extended API directly for a full-screen cursor-blinking input editor (`dbasic.asm:555-602`, `ScreenEditor` vs. plain-build's `SimpleSerialEditor`). **Conclusion: this is the one program that needs true bitmap graphics, the character-generator upload registers, and the full `$F8-$FF` bank range — everything else only needs text mode.**

### 4.4 wyrmhold (`DOS65_OS/software/wyrmhold/`)

An original top-down RPG. Per its own `README.md:6-9,143-146`: "It uses the memory-mapped video card directly for an 80x24 text display with custom 2x2 terrain and character tiles... The video and PSG access patterns follow the established 6502PC code in this repository, especially the SpeedScript screen code and the dBASIC AY-3-8910 examples." Confirmed by source: `video.asm:1-13` documents a deliberate two-path design — ordinary UI text goes through the OS driver's FARCALL API (`chrout`/`locate`, task 0, no paging needed), while the viewport/panel renderer maps physical `$F9` into logical `$A` (`vid_enter`/`vid_exit`, `video.asm:19-34`) and writes text+color cells directly (`rowbase`/`putcell`, `video.asm:36-112`) for speed — the same page-`$A` convention Speedscript uses, independently confirming §3.3. `tiles.asm:1-36` implements the "custom tiles": it uploads user-defined 8×8 glyphs into the character generator (mapping physical `$F8` into logical `$A`, writing `VideoCharGenOffset`/`VideoCharGenData`), the same registers DBASIC's `PATTERN` command exposes. `tiles.asm:11-13` documents a concrete hardware quirk: authored bitmaps use bit 7 as the leftmost pixel, but "the video character generator displays bit0 on the left," so the upload routine reverses each scanline before sending it. Per the README, each gameplay tile is four such custom characters composited in a 2×2 text-cell block for "16×16 artwork" — **this is still 100% text-mode character-cell rendering with a custom font, not a bitmap/graphics mode.** wyrmhold also drives an AY-3-8910 PSG for sound (`sound.asm`) — a separate peripheral, out of scope here (§9). **Conclusion: text mode only (80 columns) plus character-generator upload; needs physical pages `$F8` and `$F9`, same as DOS/65 and Speedscript — no LORES/HIRES register is ever touched.**

### 4.5 Summary

| Program | Text mode | Custom char-gen glyphs | True bitmap graphics | Physical pages needed | Confirmed on shipped disk image |
|---|---|---|---|---|---|
| DOS/65 (OS driver itself) | Yes (only) | No | No | `$F8`, `$F9` | N/A (the OS) |
| Speedscript | Yes (only) | No | No | `$F8`, `$F9` | Yes (`SPSC`) |
| DBASIC (plain) | Indirect, via OS console | No | No (compile-time syntax error) | None directly | Yes (`DBASIC`) |
| DBASIC (memory-mapped) | Yes | Yes (`PATTERN`) | **Yes** (LORES/HIRES/DOUBLE/QUAD/MONO) | `$F8`-`$FF` | Yes (`DBASICMP`) |
| wyrmhold | Yes (only) | Yes | No | `$F8`, `$F9` | Yes (`WYRMHOLD`) |

All four `.COM` names are drawn from `system-reference.md`'s own disk-content inventory (LBA `0x0100` CP/M directory scan) — these aren't hypothetical builds, they are present on the disk image the emulator already boots.

## 5. Current emulator implementation state (`emulator/src/`)

There is **no video-card implementation at all**. The only "video" references in `emulator/src/` are `rom.rs:3-42`/`config.rs:22,113`/`emulator.rs:34` — the M3-scope `RomBank::Video` boot-ROM-bank selector, unrelated to the display hardware itself (§3.4).

Concretely, what happens today if firmware maps the video pages exactly as designed:

- `Bus` allocates exactly 512 KiB of RAM (`bus.rs:53`, `ram: vec![0u8; 512*1024]`), covering physical pages `$00`-`$7F` only.
- The MMU (`mmu.rs:62-78`) is a generic 64-task page-translation table with no awareness of what a physical page number "means" — it will happily translate logical `$B000` (or `$A000`) to physical `$F8000` exactly as `VIDEOINIT`/`vid_enter`/`PAGE_ENTER` all expect.
- But `Bus::phys_read`/`ram_write` (`bus.rs:202-208,115-119`) both guard `phys < self.ram.len()` — physical addresses at or above `$80000` (i.e. every video page, `$F8000` and up) fall through to `self.open_bus` on read and are **silently discarded** on write.
- Consequence: `VIDEOINIT`'s presence self-test (§4.1, write `$00`/`$FF` to offset `$06`, expect readback) would read back the configured `open_bus` value instead — almost certainly **failing** the test against today's emulator (the fail path still lets boot continue; the success path `RTS`s at `bios_video.asm:115` and the fail path `RTS`s at `bios_video.asm:131`, so either outcome returns normally and would not hang DOS/65, just silently produce no visible output). No test in `emulator/tests/` currently exercises this path either way.
- There is no config field for video at all — not even a dead placeholder like `cpu_subtype`/`shadow_addr_low` (`system-reference.md` R0.1/R0.6). This is a green-field addition.
- `requirements.md:255` already scoped this out explicitly: "Video card (memory-mapped display, pages `$F8-$F9`): present in the spec but not required for DOS/65 serial operation; a separate milestone may be added after M6." This document is the investigation for that milestone.

## 6. Surfacing video output to a human: frontend options

### 6.1 Options considered

| Option | Fidelity | Dev effort | Fit with this project | Testability |
|---|---|---|---|---|
| **Web server → browser canvas** (HTTP/WebSocket streaming a framebuffer, `<canvas>` client) | Full RGB, arbitrary scaling | Medium-high: needs an HTTP/WS server crate + a small JS client + a VRAM→pixel decoder | Works from anywhere reachable over a port — no local display required | High: server can be driven/asserted against headlessly in `cargo test` with no GPU/display dependency |
| **Native window (`minifb`)** | Full RGB | Low-medium: `update_with_buffer()` on a `Vec<u32>`, same decoder as above | Simplest single-process integration into the existing `run()`/`step_one()` loop (`emulator.rs:144,196`) | Low in this environment: needs a real (or virtual, e.g. Xvfb) display backend even to construct a window; awkward in a headless CI/agent sandbox |
| **`pixels` (GPU-backed framebuffer)** | Full RGB | Medium-high: `wgpu` pipeline setup on top of the same buffer | Same native-window fit as `minifb`, heavier dependency chain and GPU/driver requirement for no fidelity gain over `minifb` at this resolution | Same headless problem as `minifb`, worse (also needs a GPU or software Vulkan/GL fallback) |
| **Terminal ANSI-art rendering** | Capped: character-cell resolution, best-effort color quantization | Medium: no new dependency (`crossterm` is already a dependency, `Cargo.toml:16`, already used for the serial console) but real effort in a bitmap→glyph/color scan-converter | Matches the project's current "headless CLI" self-image (`requirements.md:64`, "no video output required for any milestone") most closely | Same as web: no display dependency, but crude fidelity makes assertions less meaningful for graphics-mode content |
| **Headless/snapshot-only** (read VRAM bytes directly, no rendering) | N/A — not for humans | Low | This is not a competing option — it is the prerequisite substrate every other option needs, and the right first step regardless | Highest: exactly the existing gate-test style (`phys_read` assertions) already used for every other subsystem |

### 6.2 Recommendation

**Build the headless VRAM/register model first (§7, `M7`), independent of any frontend.** This lets gate tests (in the project's existing style) assert on VRAM/register bytes directly — e.g. that `VIDEOINIT`'s self-test now passes, that `WRVID` produced the right byte at the right offset — with zero rendering code and no display dependency, mirroring how M1-M6 validate every other peripheral.

**For the human-facing layer, recommend the web-frontend option the bead's own working assumption already leaned toward — but validate it for a specific, project-grounded reason rather than rubber-stamping it: this investigation, this emulator's test suite, and (per this session's own working environment) likely most future development on it all run inside headless agent sandboxes or CI with no guaranteed display server.** A native window (`minifb`/`pixels`) has nowhere to appear in that context; a web server bound to a local port is reachable from a browser on whatever machine the human is actually sitting at, including through an SSH tunnel. This is a stronger argument for this specific project than the generic "canvases render bitmaps well" reasoning, which applies about equally to `minifb`.

Terminal ANSI-art is not recommended as the primary path: three of the four named programs are text-mode only and would show correctly on the **already-working plain serial console** (`CONSOLE=$04`) with zero video-card work at all — the entire reason to build video-card support is to reach the fourth case (DBASIC's true bitmap graphics) and wyrmhold's custom tile glyphs, and a terminal can only crudely approximate exactly the content that motivates doing this in the first place. If real pixels/glyphs are going to be rendered faithfully at all, a canvas does it far more directly than an ANSI scan-converter would, for less code.

`minifb` is worth adding as a **secondary, feature-flagged** presentation layer once the framebuffer model exists (§7, `M9`) — for a developer running the emulator locally with a real display, it is less code than the web path and avoids running a server at all. Because both consume the same underlying VRAM-decode logic, adding it later is additive, not a fork.

### 6.3 Adjacent gap noted, not investigated here: keyboard input for the video console

Surfacing *output* doesn't make the video console (`CONSOLE=$13`) interactively usable — `specifications/multio-card-investigation.md` (§7, finding 4) already established that **no PS/2 scancode injection path exists at all** for the Multi-I/O keyboard controller the video console depends on for input. Watching wyrmhold or DOS/65 boot is possible with output alone; actually playing wyrmhold or typing into DBASIC's `ScreenEditor` needs that separate gap closed too. This is called out because it's a real dependency for a *fully interactive* video milestone, not because it's in scope here — it is not part of this bead's "surface output" question, and the cited document already covers it in more depth than would be appropriate to duplicate.

## 7. Proposed milestone breakdown

Same style as `plans/pc6502-emulator-milestones/decomposition.md`. Continues the existing M1-M6 numbering per `requirements.md:255`'s own "a separate milestone may be added after M6" framing. **No beads exist yet for any of these** — proposed for whoever picks this up to scope into real bead IDs.

### M7 — Video card core model: VRAM, registers, character generator (headless, no rendering)

**Req traceability:** this document §3, §5
**Expected files:**
- `emulator/src/video.rs` (new) — `VideoCard`: 32 KiB backing store for physical pages `$F8`-`$FF` (§3.2); register bank at bank-`$F8` offsets `$00`-`$0E` (§3.1 table); 256×8-byte character-generator RAM addressable via the `$02`/`$03` offset/data registers with auto-advance across the 8 rows (`bios_video.asm:26-27`, `tiles.asm:5-9`).
- `emulator/src/bus.rs`/`mmu.rs` — route physical pages `$F8`-`$FF` to `VideoCard` instead of falling through to the generic RAM-bounds check (`bus.rs:105-119,202-208` today silently drop these).
- `emulator/src/config.rs` — video-card presence/enable config (mirroring how other optional peripherals are modeled, e.g. `mmu_power_on_fill`/`open_bus`).
- `emulator/tests/m7_video_core_gate.rs` (new) — `VIDEOINIT`'s self-test (write `$00`/`$FF` to bank-`$F8` offset `$06`, read back) now passes; a `WRVID`-equivalent write lands the expected char+color byte at the `GETVIDEOADDRESS`-computed offset (`bios_video.asm:529-579`); `SPEEK`/`SPOKE`-equivalent access round-trips across all 8 banks.

**Gate observable:** direct VRAM/register reads match what firmware writes, at both the OS-driver page-`$B` convention and the application page-`$A` convention (§3.3) — same physical bytes, different logical windows.

**Explicit non-goal:** no rendering or output surface yet — this milestone only makes the hardware real to firmware/software, verified the same way every other M1-M6 peripheral was (direct memory/register assertions).

**Dependencies:** none beyond the existing MMU (M2). **Skipped:** rendering, input. **Blocked:** none.

### M8 — Human-facing output: web framebuffer viewer (text mode first)

**Req traceability:** this document §6
**Expected files:**
- A new dependency for a minimal HTTP + WebSocket server (crate choice is an implementation decision for whoever picks this up — evaluate against the workspace's existing preference for small dependency footprints, e.g. `Cargo.toml`'s current minimal set).
- `emulator/src/video_server.rs` (new) — decodes `VideoCard` state (M7) into a pixel frame on demand: text+character-generator glyphs first (covers DOS/65, Speedscript, wyrmhold, and DBASIC's non-graphics output — §4.5), served over a WebSocket to a browser `<canvas>` client (a small static HTML/JS page, no build tooling needed).
- `emulator/tests/m8_video_frontend_gate.rs` (new) — since no display/browser is available in CI (§6.1), assert against the served bytes directly: write known content into `VideoCard`, connect a test WS client (or fetch a snapshot endpoint), assert the decoded pixel bytes at known offsets match the expected glyph bitmap.

**Gate observable:** a served frame's pixel bytes match hand-computed expectations for known VRAM content — asserted without a real browser.

**Dependencies:** M7. **Skipped:** true bitmap-mode decoding (M9), input. **Blocked:** none.

### M9 — Bitmap graphics decode and native-window alternative

**Req traceability:** this document §4.3, §6.2
**Expected files:**
- `emulator/src/video_server.rs` (extend) — LORES/HIRES/DOUBLE/QUAD/MONO decode paths, using `screencmds.asm`'s `V_PLOT_LORES`/`V_PLOT_HIRES_COLOR`/`V_PLOT_HIRES_MONO` (`screencmds.asm:625-931`) as the authoritative pixel-packing reference (§3.2, §4.3) — this is the only known real consumer of these modes, so its behavior *is* the spec.
- Optional, feature-flagged: a `minifb` native-window presentation layer consuming the same `VideoCard`-decode logic as `video_server.rs`, for local desktop use (§6.2).
- `emulator/tests/m9_video_graphics_gate.rs` (new) — feed a known `DBASICMP`-style `SCREEN`/`PLOT` register sequence, assert the decoded frame matches expected pixel colors for LORES and HIRES-mono at a few coordinates.

**Gate observable:** a `PLOT`-equivalent register/VRAM sequence decodes to the expected pixel color in the served/rendered frame.

**Dependencies:** M7, M8. **Skipped:** the un-addressable tail of "DOUBLE HIRES" (§3.2 — banks beyond `$FF`), multicolor mode (§3.1 — undocumented-in-practice). **Blocked:** none.

## 8. Evidence index

- **[V1]** `PC6502_firmware_source/bios_video.asm:1-131` — full OS video driver: register/VRAM comment, `VIDEOINIT`, `SETMODE`, `CLEARSCREEN`, `SETCOLOR`, `SETXY`, `WRVID`, cursor paint/unpaint, `SCROLLUP`, `GETVIDEOADDRESS`.
- **[V2]** `PC6502_firmware_source/dos65drv.asm:15-30,41-123` — dispatch-table lookup mechanism and the full function table (0-68), including both video-related groups (19-23, 37-39/56-59).
- **[V3]** `DOS65_OS/software/dbasic/screencmds.asm:1-1249` — DBASIC's memory-mapped-screen graphics API: register/VRAM comment (superset of `[V1]`'s), `V_SCRCLR`/`V_LOCATE`/`V_COLOR`/`V_SPEEK`/`V_SPOKE`/`V_SCREEN`/`SETUPMODE0-2`/`V_PLOT` (lores/hires-color/hires-mono)/`V_PATTERN`/`LAB_LINE`, and the `.ELSE` branch showing these are compile-time syntax errors without `MEMORYMAPPEDSCREEN`.
- **[V4]** `DOS65_OS/software/dbasic/dbasic.asm:55-69,553-604` — cold-start `MEMORYMAPPEDSCREEN` conditional block (clear screen/unpaint cursor via `farfunct`), `ScreenEditor` vs. `SimpleSerialEditor` BASIC-input dispatch, cursor paint/unpaint around each input keystroke.
- **[V5]** `DOS65_OS/software/dbasic/iovect.asm:1-40` — plain-build console I/O via `PEM` calls only, no video-hardware reference.
- **[V6]** `DOS65_OS/software/dbasic/Makefile:1-14` — the two build targets (`dbasic.out` vs. `dbasicmap.out`, distinguished solely by `-D MEMORYMAPPEDSCREEN`).
- **[V7]** `DOS65_OS/software/speedscript/defines.asm:12,19-23` — `SETPAGE`/`VIDEOBANK`/`VIDTEXT_PAGE` constants, direct (non-driver-mediated) definition.
- **[V8]** `DOS65_OS/software/speedscript/screen.asm:331` and `io.asm:9-32,437-568` — driver-mediated mode/color/clear setup (`farfunct`) plus direct-VRAM bulk writes at logical `$A0xx`.
- **[V9]** `DOS65_OS/software/wyrmhold/README.md:1-146` — project description; explicit statement of text-mode-plus-custom-tiles design and its Speedscript/DBASIC lineage.
- **[V10]** `DOS65_OS/software/wyrmhold/video.asm:1-120` — dual-path design header comment, `vid_enter`/`vid_exit`/`rowbase`/`putcell` direct-VRAM viewport renderer.
- **[V11]** `DOS65_OS/software/wyrmhold/tiles.asm:1-40` — character-generator upload (`cg_enter`/`cg_exit`), bit-order-reversal hardware quirk.
- **[V12]** `emulator/src/bus.rs:31-63,92-130,198-208` — `Bus` struct, RAM allocation (512 KiB, pages `$00-$7F` only), `read_physical`/`write_physical`/`phys_read`/`ram_write`, the open-bus fallthrough that currently swallows all video-page access.
- **[V13]** `emulator/src/mmu.rs:1-168` — generic 64-task MMU; confirms no page-specific behavior exists for `$F8`+ (translation is uniform regardless of physical target).
- **[V14]** `emulator/src/rom.rs:1-45`, `emulator/src/config.rs:20-25,110-115`, `emulator/src/emulator.rs:30-40` — the unrelated M3-scope `RomBank::Video` boot-bank selector, cited to distinguish it from this document's scope (§3.4).
- **[V15]** `emulator/Cargo.toml:1-17` — current dependency set (`toml`, `serde`, `crossterm`); no GUI/web/GPU dependency exists today.
- **[V16]** `emulator/src/emulator.rs:10,66,81,144,196,220,266` — `Machine` struct and its `run`/`step_one`/`run_until_cycles` execution model, relevant to where a frontend would hook in.
- **[V17]** `specifications/hardware-spec.md:91-99,148-159,173-191,196-219` — prior video-card entry in the optional-peripheral table, the 32 KiB/`$BFFF` conflict (first flagged here), evidence tag `[E12]`.
- **[V18]** `specifications/memory-mmu.md:41,48-50,242-244,267,289,303,317-319` — prior video-page/MMU-interaction analysis, evidence tags `[E9]-[E11]` (the first citation of wyrmhold's video/tiles/defines files, independently re-verified by this document).
- **[V19]** `specifications/system-reference.md:379-385,529-551,577,621,728-749` — synthesized device table, dispatcher/console-selector tables, conflict `C8`, open question `R2.3`, evidence cross-reference.
- **[V20]** `docs/investigation/io-registers.md:306-308,366` — explicit non-duplication note deferring video-card detail to this investigation.
- **[V21]** `plans/pc6502-emulator-milestones/requirements.md:64-65,255` — current "no video output required" framing and the explicit "separate milestone after M6" deferral this document answers.
- **[V22]** `plans/pc6502-emulator-milestones/decomposition.md:195-227` — `WI-M3` entry, used as the structural template for §7's proposed milestones.
- **[V23]** `specifications/multio-card-investigation.md` (full document, especially §7 finding 4) — prior-investigation lesson applied per this bead's instructions (§2), and the source of the keyboard-input adjacent-gap note (§6.3).

## 9. Scope note: other findings, not investigated further

Per this investigator's scope rules, these are mentioned but not acted on:

- **wyrmhold and DBASIC both drive an AY-3-8910 PSG for sound** (`wyrmhold/sound.asm`, `dbasic.asm:55-57`'s `psginit`/`clrpsg`). This is a distinct peripheral with, as far as this investigation went, no emulator implementation either — completely out of scope for a *video* investigation, but a natural sibling gap if audio ever becomes a goal.
- **`bios_video.asm`'s own `VIDEOINIT_FAIL` path (`bios_video.asm:117-131`) appears to print the same message text as the success path** — `VIDEOMESSAGE2` ("NOT ", `bios_video.asm:591-592`) is defined but never actually loaded into `STRPTR` by either branch. This looks like a latent cosmetic bug in the original firmware source (the "not found" banner never actually says "NOT"), not an emulator concern — noted in case whoever implements M7 wonders why both self-test outcomes look identical when testing against real ROM output.
- **Register `$0E`/multicolor mode and the "Text/Color Page 2" VRAM region are documented but functionally unexercised** by every piece of software reviewed (§3.1, §3.2) — flagged in §7 (M9) as explicitly out of scope rather than repeated here.

## Closing

Investigation complete per all five acceptance items. Closing bead `mc-ehx` with a summary pointing here.
