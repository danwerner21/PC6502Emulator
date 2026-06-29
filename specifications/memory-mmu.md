# PC6502 Memory and MMU Specification

**Investigation:** `mc-2b3`  
**Snapshot reviewed:** 2026-06-27  
**Purpose:** Define the memory behavior an emulator must expose, while separating confirmed circuit/software contracts from assumptions that still require a board test.

## 1. Scope and terminology

The 6502 always issues a 16-bit **CPU-visible** or **logical** address in `$0000-$FFFF`. The MMU and board decoder produce a 20-bit **physical** address, `$00000-$FFFFF`. A **logical page** is one 4 KiB CPU range selected by logical address bits 15-12. A **physical page** is an 8-bit selector that becomes physical address bits 19-12.

For an MMU-enabled access:

- `logical_page` is CPU address bits 15-12, in the range `$0-$F`.
- `offset` is CPU address bits 11-0.
- `physical_page` is the byte in the active task's map entry for `logical_page`.
- The resulting physical address is `physical_page × $1000 + offset`.

Thus page value `$10` maps a logical page to physical `$10000-$10FFF`, and page value `$F8` maps it to physical `$F8000-$F8FFF`. This distinction is essential: task numbers select maps; page values select physical address space.

Evidence confidence used below:

- **Confirmed:** directly stated or wired in the board schematic, board documentation, or exercised by checked-in firmware.
- **Inferred:** required to reconcile multiple confirmed facts, but not directly stated.
- **Unknown:** no source in the reviewed material defines the behavior.

## 2. Physical address space

### 2.1 Address width and installed RAM

The MMU map store supplies physical address bits 19-12, while CPU bits 11-0 pass through. The physical address space is therefore 1 MiB, divided into 256 pages of 4 KiB [E3]. The baseboard contains one 512 KiB SRAM [E1, E4]. Firmware explicitly treats pages `$00-$11` as RAM, so low physical pages are RAM in the PC6502 configuration [E5].

An emulator must not equate the 1 MiB decode space with 1 MiB of installed RAM. Page selectors `$80-$FF` remain meaningful even though they are outside the 512 KiB SRAM capacity. They may select expansion hardware, an I/O/ROM overlay, or an unclaimed physical hole.

### 2.2 Physical decode regions

| Physical range/page | Device or behavior | Status |
|---|---|---|
| `$00000-$7FFFF` | 512 KiB main SRAM, except where board I/O/ROM shadow decode takes precedence | Confirmed capacity; low placement is required by firmware [E1, E3-E5] |
| Physical shadow I/O page | On-board and slot I/O; the PC6502 software-visible default is logical `$E000-$EFFF` | Confirmed overlay; exact high physical prefix strap is not recorded locally [E2-E4] |
| Physical shadow boot page immediately above I/O | 4 KiB selected ROM window | Confirmed overlay [E2-E4] |
| Page `$F8` and at least `$F9` | Optional memory-mapped video-card control/VRAM pages | Confirmed software use, not a baseboard RAM claim [E9-E11] |
| Other pages outside installed RAM | Expansion response or unclaimed hole | Unknown until a responding card is configured |

The V1.1 memory-map schematic explicitly says the I/O and ROM boot spaces overlay RAM, not a separate ROM-only address universe [E4]. Device decode therefore has priority over the underlying SRAM for matching physical addresses. A write to the ROM overlay must not alter hidden SRAM. An I/O write reaches the selected device. Unclaimed physical reads and writes are discussed in section 9.

The schematic also has a `SHADOW ADDR` configuration and separate high-address signals used while the MMU is disabled [E4]. The reviewed repository does not state the installed jumper positions. The firmware creates page entries `$0E` and `$0F` and then continues to access I/O at `$E000` and ROM at `$F000`; consequently, a firmware-compatible emulator default must make physical pages `$0E` and `$0F` select the documented I/O and ROM windows. Any configurable nonzero shadow prefix is an advanced hardware option, not the default for this software image (**inferred**).

### 2.3 Optional video range contradiction

The video driver calls the card a 32 KiB area, starts it at physical page `$F8`, and exercises pages `$F8` and `$F9` [E9]. Its comments describe offsets through `$BFFF`, which would exceed 32 KiB, and a full `$F8-$FF` 32 KiB span may collide with a high-prefix I/O/ROM shadow configuration. Only pages `$F8` and `$F9` are proven by the reviewed software. Do not allocate all `$F8-$FF` to video without a separate video-card specification.

## 3. CPU-visible maps

### 3.1 Reset and MMU-disabled map

The board reset clears the MMU-enable latch. Firmware also begins `INITPAGES` by explicitly writing zero to the enable register because disabled is expected but important enough to enforce [E3, E5]. At the reset vector fetch:

| CPU-visible range | Required default response |
|---|---|
| `$0000-$DFFF` | Direct/default RAM mapping |
| `$E000-$EFFF` | I/O overlay |
| `$F000-$FFFF` | Selected 4 KiB ROM bank |

The board document prints RAM as `$0000-$E000`, overlapping the separately documented I/O start. This specification normalizes the non-overlapping RAM endpoint to `$DFFF` [E2].

When the MMU is disabled, the map SRAM does not translate accesses. CPU bits 15-12 pass through the disabled-path multiplexer, with the board's shadow high-address configuration supplying the upper physical bits [E3, E4]. For the firmware-compatible default, logical `$1234` therefore reaches physical `$01234`, `$E123` reaches the physical I/O overlay, and `$FFFC` reaches the ROM reset-vector location.

The active-task latch still exists while translation is disabled. A write to the active-task register while disabled selects the task that will take effect on the next enable; firmware relies on this ordering [E5].

### 3.2 MMU-enabled map

Each task has 16 independent page bytes, one for each logical 4 KiB page. The active task applies to the entire 64 KiB CPU space, including zero page, stack, I/O, ROM/vector addresses, instruction fetches, reads, and writes [E3, E5]. There is no evidence for a page-level read-only or write-protect bit: all eight map-entry bits form the physical page number.

After translation, the physical decoder decides whether the access reaches SRAM, I/O, ROM, an expansion device, or a hole. This ordering explains the firmware's `$xFE0`/`$xFDx` notation: I/O can appear in whichever logical page maps the physical I/O page. The normal firmware preserves I/O and ROM in logical pages `$E` and `$F` by using entries `$0E` and `$0F` in tasks 0 and 1 [E5].

Consequences:

- Two logical pages may map the same physical page. Reads and writes through either alias observe the same backing storage/device.
- Switching tasks can change zero page and stack immediately, not only the `$C000-$DFFF` bank used by DOS/65.
- Mapping logical page `$E` away from the physical I/O page removes the MMU registers from `$EFD0-$EFEF`.
- Mapping logical page `$F` away from the physical boot page removes ROM and hardware vectors from `$F000-$FFFF` for that task.
- If a new task does not preserve the page containing the currently executing code and the stack page, the instruction after the task-register write and later stack accesses occur in the new mapping.

## 4. Task and map-store format

| Property | Value | Evidence |
|---|---:|---|
| Number of task contexts | 64 (`$00-$3F`) | Firmware statement and six-bit task latches [E3, E5] |
| Entries per task | 16 | One per logical page `$0-$F` [E3, E5] |
| Entry width | 8 bits | Map SRAM data becomes physical address bits 19-12 [E3] |
| Page size | 4 KiB | CPU bits 11-0 pass through [E3, E5] |
| Total architected map bytes | 1,024 | 64 × 16 |

The schematic uses a larger commodity SRAM for the map store, but only the six task bits and four logical-page bits are architecturally selected. Unused capacity is not additional software-visible tasks [E3].

Task writes use only data bits 5-0; bits 7-6 are not task-number bits. Emulator task selectors must mask values to `$3F`. Map-entry writes use all eight data bits without masking.

## 5. MMU register map

The normal addresses below assume I/O is visible in logical page `$E`. If the physical I/O page is mapped into another logical page `$x`, the same offsets appear at `$xFD0-$xFEF`.

### 5.1 Task-map edit window

| CPU address | Access | Meaning |
|---|---|---|
| `$EFD0-$EFDF` | Read/write | Sixteen map entries for the task selected by `$EFE1`; low address nibble selects logical page `$0-$F`; the byte is the physical page selector |

The edit selector and active selector are independent. Editing task 1 does not activate task 1. The window is usable while the MMU is disabled; BIOS initialization and `SETPAGE` require that behavior [E5]. A window read returns the selected entry. A window write changes future translations for that task and changes current translation immediately if the edited task is active and the MMU is enabled.

### 5.2 Control and status offsets

| Address | Access | Bits and effect | Reset/undefined behavior |
|---|---|---|---|
| `$EFE0` | Write only | Active-task selector; data bits 5-0 select task `$00-$3F`; bits 7-6 ignored | Reset clears active task to 0 [E3, E5] |
| `$EFE1` | Write only | Map-setup selector; data bits 5-0 select the task exposed at `$EFD0-$EFDF`; bits 7-6 ignored | Setup selector has no documented reset value; software must write it before using the edit window [E3, E5] |
| `$EFE2` | Write only | MMU enable latch; data bit 0 `0` bypasses translation and `1` enables translation; other bits have no documented effect | Reset clears enable to 0 [E3, E5] |
| `$EFE3` | Unassigned | No defined register | Read value/open-bus behavior unknown; writes have no defined effect |
| `$EFE4` | Read only | Bits 5-0 return active task; bit 7 returns MMU-enable status in the V1.1 schematic; bit 6 is not assigned | Writes have no defined effect [E3, E5] |
| `$EFE5` | Unassigned | No defined register | Read value/open-bus behavior unknown; writes have no defined effect |
| `$EFE6` | Read-side-effect | A read asserts the expansion-bus ISA terminal-count (`TC`) signal; no useful returned data is defined | Pulse width, clearing, and read data are unknown [E3, E5] |
| `$EFE7` | Read only | Low four bits report the current I/O-page value; upper four bits are not meaningful | Firmware does not use it; exact interpretation under nondefault shadow straps remains unresolved [E3-E5] |
| `$EFE8-$EFEF` | Unassigned | No defined registers | Read value/open-bus behavior unknown; writes have no defined effect |

The firmware comment “hit ISA TC Bit” at `$EFE6` is ambiguous in isolation. The V1.1 schematic exposes `TC` as an output generated by the MMU/control decode, supporting a read-triggered bus side effect rather than a readable status bit [E3, E5].

The board document allocates the whole `$EFE0-$EFEF` block, but allocation is not mirroring. No evidence shows any implemented register mirrored at an unassigned offset or outside this 16-byte block.

## 6. Reset versus BIOS-initialized state

These two states must not be conflated.

### 6.1 Hardware reset state

- MMU enable is 0, so translation is bypassed [E3].
- Active task is 0 [E3].
- The setup-task selector is not established by the reviewed sources.
- Map SRAM contents are not reset and must be treated as indeterminate.
- ROM is visible at logical `$F000-$FFFF`, allowing vector fetch at `$FFFC-$FFFD` [E2-E6].

An emulator may use deterministic bytes internally for reproducibility, but software must not be promised any reset map contents before firmware initializes them.

### 6.2 State after `INITPAGES`

BIOS disables the MMU, initializes task 1, initializes task 0 last, selects task 0, and enables the MMU [E5].

| Logical page | CPU range | Task 0 physical page | Task 1 physical page |
|---:|---|---:|---:|
| `$0` | `$0000-$0FFF` | `$00` | `$00` |
| `$1` | `$1000-$1FFF` | `$01` | `$01` |
| `$2` | `$2000-$2FFF` | `$02` | `$02` |
| `$3` | `$3000-$3FFF` | `$03` | `$03` |
| `$4` | `$4000-$4FFF` | `$04` | `$04` |
| `$5` | `$5000-$5FFF` | `$05` | `$05` |
| `$6` | `$6000-$6FFF` | `$06` | `$06` |
| `$7` | `$7000-$7FFF` | `$07` | `$07` |
| `$8` | `$8000-$8FFF` | `$08` | `$08` |
| `$9` | `$9000-$9FFF` | `$09` | `$09` |
| `$A` | `$A000-$AFFF` | `$0A` | `$0A` |
| `$B` | `$B000-$BFFF` | `$0B` | `$0B` |
| `$C` | `$C000-$CFFF` | `$0C` | `$10` |
| `$D` | `$D000-$DFFF` | `$0D` | `$11` |
| `$E` | `$E000-$EFFF` | `$0E` | `$0E` |
| `$F` | `$F000-$FFFF` | `$0F` | `$0F` |

The final control state is active task 0, setup selector task 0, and MMU enabled. Tasks 2-63 remain indeterminate. The source comment saying tasks 2-15 are open for use is a usage suggestion, not a reduction of the hardware's 64-task capacity [E5].

## 7. Task switching and software-visible effects

### 7.1 DOS/65 driver bank

With task 0 active, logical `$C000-$DFFF` addresses physical `$0C000-$0DFFF`. With task 1 active, the same logical addresses reach `$10000-$11FFF`. The loader copies DOS/65 to task-0 logical `$B800-$DFFF`, switches to task 1, copies the banked drivers to logical `$C000-$DFFF`, returns to task 0, and jumps to `$B800` [E7]. BIOS far calls switch to task 1, call the dispatcher at logical `$C000`, and restore task 0 [E5, E6].

The rest of task 0 and task 1 remains identical after initialization, which keeps ROM code, vectors, zero page, stack, and I/O usable across those switches.

### 7.2 Active-task write timing

An `$EFE0` write changes the active task latch immediately. The write itself is decoded using the old mapping; the next bus cycle uses the new mapping when enabled. Emulator implementations should apply the task change at the end of the register-write bus cycle.

### 7.3 MMU enable timing

Writing zero to `$EFE2` makes the next bus cycle use the direct/reset mapping. Writing one makes the next bus cycle use the selected task map. The map SRAM and task latches retain their values across disable/enable operations. Only hardware reset establishes enable 0 and active task 0; map contents persist as ordinary volatile SRAM until power loss or overwrite.

## 8. BIOS `SETPAGE` service and its edge cases

ROM entry `$FFF6` jumps to `SETPAGE`. Its calling convention is A = task, X = logical page, and Y = physical page [E5, E6, E10, E11]. The routine:

1. Saves A, disables the MMU, and selects A as the setup task.
2. Writes Y to edit-window entry X.
3. Selects setup task 0 and writes X to task 0 entry X, restoring that task-0 page to identity.
4. Enables the MMU and returns.

Important consequences:

- It leaves the setup selector at task 0.
- It does not change the active-task selector, so the previously selected task becomes active again when MMU is re-enabled.
- It does not preserve a previously disabled MMU state; it always returns enabled.
- It does not preserve a nonidentity task-0 entry at X.
- Calling it with A = 0 cannot create a nonidentity task-0 mapping because step 3 overwrites the requested Y with X.
- X must be `$00-$0F`. The routine does not mask or validate X; larger values make the indexed store escape `$EFD0-$EFDF` and may write MMU control registers or unrelated I/O.
- A is effectively masked to six task bits by hardware. Y uses all eight bits.
- The routine does not disable interrupts, so interrupt behavior during its MMU-disabled interval is not made atomic by the service itself.

The task-0 identity rewrite is a compatibility workaround. Source comments say that on some boards every edit-window write also writes task 0, so BIOS initializes task 1 first and task 0 last, and `SETPAGE` repairs the affected task-0 entry [E5]. This side effect is revision-dependent rather than part of the clean architectural model. An emulator should implement normal selected-task-only writes by default and may offer a named compatibility option for the task-0 alias defect.

## 9. Reads, writes, mirrors, and holes

### 9.1 RAM and aliases

RAM reads return the last stored byte. RAM writes update the selected physical byte. Logical aliases and aliases across tasks are coherent because they address the same physical byte.

### 9.2 ROM

ROM reads return the selected 4 KiB bank. ROM writes have no defined storage effect and must not fall through into hidden SRAM. The physical K1 jumper selects one of two 4 KiB halves from the supplied 8 KiB programming image; exact power-on jumper position is external configuration [E1, E2].

### 9.3 I/O and MMU windows

I/O reads and writes are dispatched after physical translation. The map window supports both reads and writes. Write-only MMU registers do not have defined readback aliases; status is at `$EFE4`. Reads of `$EFE6` have a TC side effect. Writes to read-only or unassigned offsets have no documented effect.

The board documentation reserves `$EFA0-$EFCF` and `$EFF0-$EFFF` as open, and leaves several `$EFE0-$EFEF` offsets unassigned [E2]. There is no evidence that these holes mirror neighboring hardware.

### 9.4 Unclaimed physical space and open bus

No reviewed source defines the byte returned when no RAM, ROM, I/O, or expansion device drives the bus. It also does not define whether the value is a pull-up pattern, the previous data-bus value, or electrically unstable. Emulator policy should therefore be explicit and configurable. A deterministic initial policy such as `$FF` is acceptable for bring-up, but it must be labeled an emulator policy, not a hardware fact. Writes to an unclaimed address have no target and should be discarded.

## 10. Transition examples

### 10.1 Reset to normal BIOS operation

1. Reset forces MMU disabled and active task 0.
2. The CPU fetches the reset vector from ROM at `$FFFC-$FFFD`.
3. BIOS initializes task 1 and task 0 through the edit window while translation is disabled.
4. BIOS selects task 0 and enables translation.
5. Because task 0 is identity mapped, the CPU-visible map remains apparently unchanged, but subsequent task switches can expose banked physical RAM.

### 10.2 Far call into banked drivers

1. ROM executes in task 0 and writes 1 to `$EFE0`.
2. Logical `$C000` changes from physical `$0C000` to `$10000`.
3. The dispatcher runs from physical page `$10` while zero page, stack, I/O, and ROM remain shared.
4. The return path writes 0 to `$EFE0`, restoring task-0 `$C000`.

### 10.3 Video page mapping

The video driver calls `SETPAGE` with task 1, logical page `$B`, and physical page `$F8` for registers/character-generator access or `$F9` for text/color memory [E9-E11]. Task 0 page `$B` remains `$0B`. After task 1 is activated, `$B000-$BFFF` reaches the selected video page; returning to task 0 restores ordinary RAM at the same logical range.

### 10.4 Aliased pages

If task 2 entries `$4` and `$9` both contain `$22`, CPU `$4123` and `$9123` both address physical `$22123`. A write through either address must be observable through the other.

### 10.5 Losing I/O or vectors

If an active task maps logical page `$E` to ordinary RAM, `$EFE0` no longer selects the MMU register and software may be unable to switch tasks through the normal address. If logical page `$F` is mapped away from the boot page, IRQ/NMI vector fetches use the replacement physical page. Software must preserve or deliberately relocate those pages.

## 11. Emulator-facing requirements

An implementation conforming to the evidence in this snapshot must:

1. Separate the 64 KiB logical CPU space from a 20-bit, 1 MiB physical decode space.
2. Model 64 tasks, 16 entries per task, and 8-bit physical page selectors.
3. Bypass page translation after reset and initialize only enable = 0 and active task = 0; do not promise reset map contents.
4. Implement the exact edit/control/status offsets in section 5, including task masking, enable bit 0, `$EFE4` status, and `$EFE6` read side effect.
5. Apply translation before physical RAM/ROM/I/O/expansion decode.
6. Provide 512 KiB of installed base RAM without turning all remaining physical pages into RAM.
7. Give physical I/O and ROM overlays precedence over hidden SRAM.
8. Preserve coherent aliases when multiple logical mappings select the same physical location.
9. Support the task 0/task 1 BIOS maps and task switches used by the DOS/65 loader and far-call dispatcher.
10. Treat video pages and other expansion responses as optional devices.
11. Expose open-bus/unmapped-read and shadow-prefix choices as explicit emulator policy/configuration until board measurements settle them.
12. Keep the task-0 edit alias bug disabled by default and, if implemented, identify it as a board-revision compatibility mode.

## 12. Verification matrix for implementers

| Test | Required observation |
|---|---|
| Reset | Enable reads disabled, active task reads 0, reset vector comes from ROM, map bytes are not relied upon |
| Disabled bypass | Changing map entries does not alter CPU translation until enabled |
| Map round trip | Select a setup task, write all 16 bytes through `$EFD0-$EFDF`, and read the same bytes back |
| Task mask | Writing `$FF` as active/setup task selects task `$3F` |
| Full-page translation | Mappings affect instruction, data, zero-page, stack, and vector cycles consistently |
| Task alias | Two logical pages mapped to one RAM page remain coherent |
| Task isolation | Equal logical addresses in tasks 0 and 1 can reach different physical pages |
| Status | `$EFE4` reports active task in bits 5-0 and enable status in bit 7 |
| Enable transition | Disable uses direct mapping on the following cycle; re-enable restores the selected task |
| I/O translation | Mapping the physical I/O page into another logical page relocates device/register offsets |
| ROM precedence | ROM reads beat hidden SRAM and ROM writes do not modify it |
| Hole policy | Reads follow the configured open-bus policy and writes are discarded |
| BIOS initialization | Final maps exactly match section 6.2 |
| DOS/65 bank | Task 0 `$C000` resolves to `$0C000`; task 1 `$C000` resolves to `$10000` |
| Video mapping | Task 1 can map `$B000` to `$F8000` and `$F9000` without allocating those pages as base RAM |
| `SETPAGE` bounds | Valid X is `$0-$F`; tests must reject or deliberately exercise the register-corrupting out-of-range behavior |

## 13. Contradictions and unresolved questions

| Topic | Evidence conflict or gap | Required treatment |
|---|---|---|
| RAM endpoint | Board text says `$0000-$E000`, overlapping I/O at `$E000` [E2] | Interpret default RAM as `$0000-$DFFF` |
| Reset map bytes | Map store is SRAM and no initialization is documented before BIOS writes tasks 0/1 [E3, E5] | Treat map contents as indeterminate even though translation is safely disabled |
| Shadow prefix | Schematic exposes `SHADOW ADDR` and disabled-path high bits, but local documentation gives no jumper setting [E4] | Default to the firmware-compatible low pages; make other straps configurable |
| `$EFE7` | Firmware says “current IO page” with only four meaningful bits; schematic describes I/O-prefix readback, but no software consumes it [E3-E5] | Return the documented default page value; do not invent additional bits or relocation semantics without board validation |
| Task-0 alias | Firmware says only “some boards” duplicate edit writes into task 0 [E5] | Normal behavior by default; optional compatibility quirk |
| Register read values | Readback of write-only and unassigned offsets is not defined | Use the emulator's named open-bus policy |
| `$EFE6` timing | TC is a schematic output and read-side effect, but pulse duration and downstream behavior are not specified [E3, E5] | Model an observable read event; defer cycle-accurate pulse timing |
| Video span | 32 KiB claim conflicts with documented offsets through `$BFFF`, and only pages `$F8/$F9` are exercised [E9] | Implement only separately specified video pages/modes |
| Physical holes | Expansion slots receive a wider address space, but no exhaustive card decode exists | Do not silently back holes with RAM |
| MMU access after remap | Hardware permits mapping I/O away; no rescue register is documented | Preserve the trap; debugging APIs may recover state out of band |

## 14. Evidence index

- **[E1]** `documentation/PC6502_system_documentation.md:1-24` — 512 KiB RAM, 4 KiB ROM, MMU, slots, and ROM-bank jumper.
- **[E2]** `documentation/PC6502_system_documentation.md:28-41` — default CPU-visible RAM, I/O, ROM, edit-window, and register allocations.
- **[E3]** Upstream 6502PC V1.1 [`MMU-4.kicad_sch`](https://github.com/danwerner21/6502PC/blob/99d730026bddbc4e980c204e43d698dc378fb4b9/MMU-4.kicad_sch) — 20-bit output address path, map SRAM, six-bit active/setup task latches, reset-cleared enable/active state, map read/write paths, status readback, and TC output. Commit `99d7300`, 2025-12-21.
- **[E4]** Upstream 6502PC V1.1 [`memory-map.kicad_sch`](https://github.com/danwerner21/6502PC/blob/99d730026bddbc4e980c204e43d698dc378fb4b9/memory-map.kicad_sch) — 512 KiB SRAM, shadow I/O/ROM physical decode, disabled-MMU high-address path, ROM selection, and I/O-prefix readback. Commit `99d7300`, 2025-12-21.
- **[E5]** `PC6502_firmware_source/bios_pager.asm:8-58,74-104` — register descriptions, task count, BIOS maps, initialization order, task-0 alias warning, page switching, and `SETPAGE` behavior.
- **[E6]** `PC6502_firmware_source/6502PCbios.asm:15-18,26-88,215-228` — ROM placement, reset flow, paging initialization, far-call/page service entries, and vectors.
- **[E7]** `PC6502_firmware_source/loader.asm:8-16,26-86` — DOS/65 task-0 and driver task-1 copy flow.
- **[E8]** `PC6502_firmware_source/bios_defines.asm:6-15` — PC6502 I/O, shadow ROM, dispatcher, and MMU addresses.
- **[E9]** `PC6502_firmware_source/bios_video.asm:16-62,134-203` — video page `$F8`, 32 KiB/range claims, and mappings of `$F8/$F9` into logical `$B`.
- **[E10]** `DOS65_OS/software/wyrmhold/defines.asm:23-49` and `DOS65_OS/software/wyrmhold/video.asm:1-34` — ROM `SETPAGE` calling convention and direct mapping of video page `$F9` into task-1 logical page `$A`.
- **[E11]** `DOS65_OS/software/wyrmhold/tiles.asm:18-36` — direct mapping of video page `$F8` into task-1 logical page `$A`.
- **[E12]** `specifications/hardware-spec.md:39-77,115-131,148-159,182-193` — earlier repository-wide baseline, conflicts, and minimum emulator contract.

## 15. Hardware validation still needed

The following measurements would remove the remaining emulator policy choices:

1. Record P1 `SHADOW ADDR` and K1 ROM-select jumper positions on a known-working PC6502.
2. Read `$EFE4` with several active tasks and both enable states to confirm bit 7 and the undriven value of bit 6.
3. Read `$EFE3`, `$EFE5`, and `$EFE8-$EFEF` after controlled preceding bus values to characterize open bus.
4. Read `$EFE7` under each shadow-address strap setting.
5. Observe `TC` while reading `$EFE6` and measure pulse polarity/duration.
6. Test map writes to task 1 while monitoring task 0 to identify whether the board revision has the alias defect.
7. Map representative absent pages (`$80`, `$F0`, `$FF`) and determine which devices, if any, respond.
8. Map the physical I/O and boot pages into nondefault logical pages to verify translation-versus-overlay ordering and vector behavior.
