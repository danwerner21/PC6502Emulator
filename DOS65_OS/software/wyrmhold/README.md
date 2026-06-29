# Wyrmhold

Wyrmhold is an original top-down fantasy RPG for the **6502PC** running
**DOS/65**, written in 6502 assembly using ca65 syntax.

It uses the memory-mapped video card directly for an 80x24 text display with
custom 2x2 terrain and character tiles, and the AY-3-8910 PSG for the title
theme and in-game sound effects. The game selects 80-column text mode at
startup before using its direct-VRAM renderer.

All maps, monsters, text, tiles, sound effects, and gameplay systems here are
original work for this project.

## Premise

King Aldren of Wyrmhold needs the lost Wyrm Key before the dragon's lair can be
opened. Explore the overworld, gather supplies, question townsfolk, cross the
Sunken March, defeat the Wyrm Warden, unlock the lair, and return after the
dragon falls.

The current build is aimed at a compact 30-45 minute run: one overworld, two
distinct towns, a castle, the Sunken Shrine, a dragon dungeon, authored
discoveries, roaming encounters, shops, equipment, consumable potions, and a
complete ending.

## Controls

| Key | Action |
|-----|--------|
| `W` `A` `S` `D` | Move north, west, south, east |
| `H` `J` `K` `L` | Alternate movement keys |
| `T` | Talk, search, or use the current location |
| `G` | Guard for one turn and reduce incoming melee damage |
| `P` | Drink a stored healing potion when wounded or poisoned |
| `?` | Open the field guide without consuming a turn |
| `Q` | Quit to DOS/65 after confirmation |
| `1`-`5` | Buy from a town shop |
| `X` | Leave a town shop |

Bump into a monster to attack it. Most successful actions consume a turn;
blocked actions and the field guide do not.

## Gameplay Features

- A scrolling 22x10-tile overworld viewport keeps the hero centered while the
  world scrolls.
- Each gameplay tile is built from four custom characters for 16x16 artwork.
- The side panel shows HP, level, XP, gold, provisions, weapon, armor, potions,
  current location or region, objective, and important status warnings.
- Terrain matters: roads conserve provisions, forests conceal the player,
  hills strengthen attacks, mountains slow travel, and marshes can poison.
- Water and town canals use restrained stable texture variants to avoid a flat
  repeated grid without becoming noisy.
- Authored overworld discoveries provide supplies, vitality, route clues, and a
  dangerous reed-ford shortcut.
- Dungeon chests use a reward table for gold, provisions, healing, lore XP,
  poison cure, stored potions, and rare equipment upgrades.
- Weapons and armor have distinct traits. Daggers can critically strike, swords
  are reliable, axes swing hard but unevenly, leather resists venom, chain is
  balanced, and plate gives the best defense at higher upkeep.
- Monsters have distinct behaviors: snakes surge and poison, skeletons wake
  from guard posts, thieves steal and flee, trolls regenerate, and the dragon
  telegraphs its breath lane.
- Eastmere and Valehaven have different maps, colors, shop inventories, prices,
  and rumor text.
- The field guide explains controls, terrain, shop traits, warnings, and the
  current objective.
- Victory and defeat screens include short narrative closure and compact
  6502PC/DOS-65 credits.

## Building

Build on a machine with cc65, `srec_cat`, and the CP/M image tools
`cpmcp`/`cpmrm` installed:

```sh
cd software/wyrmhold
make
```

The Makefile assembles with `ca65`, links with `ld65` using `dos65.cfg`, builds
`wyrmhold.com`, and copies it into:

```text
../../bin/6502PC/DOS65_6502PC.IMG
```

The original editing workstation may not have the full Linux build toolchain,
so the normal workflow is to edit locally and build on the remote Linux server.

## Validation

Run the static validator from the Wyrmhold folder:

```sh
make validate
```

The validator checks map dimensions, tile tables, metatile records, glyph
allocation, source-map characters, authored discovery coordinates, UI text
layout, important sound-effect wiring, and the linked `$9000` memory-budget
target. The Makefile defaults to `python3`; set `PYTHON=...` if your host needs
a different launcher.

Hardware and playthrough results are tracked in `HARDWARE_TESTS.md`. Planned
feature work and release-quality polish are tracked in `improvements.md`.

## Running

Boot the DOS/65 disk image on the 6502PC or emulator and run:

```text
WYRMHOLD
```

The title screen shows the current development version. Press a key to begin.

## Source Layout

| File | Purpose |
|------|---------|
| `wyrmhold.asm` | Entry point, title screen, main loop, win/lose flow, includes |
| `defines.asm` | System equates, colors, tile constants, zero-page, game state |
| `macro.asm` | 16-bit helper macros and far-call helpers |
| `sound.asm` | PSG setup, title theme, and sound effects |
| `tiles.asm` | Custom 8x8 character-generator bitmap data |
| `metatiles.asm` | 2x2 terrain, landmark, player, and monster artwork |
| `video.asm` | Video paging, viewport rendering, panel rendering, text output |
| `world.asm` | Overworld, town, dungeon, castle, shrine maps and tile tables |
| `rng.asm` | Pseudo-random generator and title timing entropy |
| `ui.asm` | Message log, stat panel, prompts, field guide, number printing |
| `entity.asm` | Monster state, spawning, regional encounters, AI behavior |
| `player.asm` | Player state, movement, map transitions, discoveries, rewards |
| `combat.asm` | Bump combat, monster attacks, status effects, leveling, messages |
| `town.asm` | Town interiors, shop menus, prices, rumor text |
| `castle.asm` | Castle audience chamber and main quest progression |
| `shrine.asm` | Sunken Shrine transition and Wyrm Key reward |
| `title.asm` | Large title-screen glyph artwork |
| `tools/validate.py` | Static project validator |
| `HARDWARE_TESTS.md` | Running hardware verification and balance checklist |
| `improvements.md` | Improvement roadmap and implementation notes |

The video and PSG access patterns follow the established 6502PC code in this
repository, especially the SpeedScript screen code and the dBASIC AY-3-8910
examples. Tile bitmap sources use bit 7 for the left edge; the upload routine
reverses each scanline for the video card's bit-0-left character generator.
