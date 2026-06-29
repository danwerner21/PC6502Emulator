# Wyrmhold Hardware Test Backlog

Use this file as the running verification list while development continues
without local 6502PC access.

Status values:

- **Passed:** verified by a remote build or on real hardware.
- **Pending:** implemented but not yet exercised on real hardware.
- **Retest:** previously passed behavior affected by later changes.
- **Failed:** defect reproduced; add a short note and leave it listed.

## Build And Startup

| Status | Test |
|--------|------|
| **Passed** | Persistent-encounter source built successfully remotely (June 15, 2026). |
| **Passed** | Title-timing RNG build link map ends at `$7337`, below the `$9000` target. |
| **Passed** | Local static validator completes with no errors; known legacy conditions remain warnings. |
| **Passed** | Run `make validate` on the remote Linux build host. |
| **Passed** | Title-timing RNG revision built successfully remotely (June 15, 2026). |
| **Passed** | In-game-help revision built successfully remotely (June 15, 2026). |
| **Passed** | Reed-ford shortcut revision built successfully remotely (June 16, 2026). |
| **Passed** | Varied chest rewards revision built successfully remotely (June 17, 2026). |
| **Passed** | Shop equipment-effects revision built successfully remotely (June 17, 2026). |
| **Passed** | Town-rumor interaction revision built successfully remotely (June 18, 2026). |
| **Passed** | Town-palette revision built successfully remotely (June 18, 2026). |
| **Passed** | Water-shimmer presentation revision built successfully remotely (June 18, 2026). |
| **Passed** | Overworld row-width cleanup revision built successfully remotely (June 18, 2026). |
| **Passed** | Metatile-label cleanup revision built successfully remotely (June 18, 2026). |
| **Passed** | Duplicate-metatile-art cleanup revision built successfully remotely (June 18, 2026). |
| **Passed** | Title-version-label revision built successfully remotely (June 19, 2026). |
| **Passed** | Victory-screen-polish revision built successfully remotely (June 19, 2026). |
| **Passed** | Defeat-screen-polish revision built successfully remotely (June 19, 2026). |
| **Passed** | Field-guide-version-label revision built successfully remotely (June 19, 2026). |
| **Passed** | Interaction-text-polish revision built successfully remotely (June 19, 2026). |
| **Passed** | Potion-consumable revision built successfully remotely (June 19, 2026). |
| **Passed** | Potion-shop revision built successfully remotely (June 19, 2026). |
| **Passed** | Shop-feedback-sfx revision built successfully remotely (June 19, 2026). |
| **Passed** | Poison-countdown-status revision built successfully remotely (June 20, 2026). |
| **Passed** | Low-food-status revision built successfully remotely (June 20, 2026). |
| **Passed** | Low-health-status revision built successfully remotely (June 20, 2026). |
| **Passed** | Shop-panel-refresh revision built successfully remotely (June 20, 2026). |
| **Passed** | Status-warning-color revision built successfully remotely (June 20, 2026). |
| **Passed** | Field-guide-status-warning revision built successfully remotely (June 20, 2026). |
| **Passed** | Field-guide-shop-info revision built successfully remotely (June 20, 2026). |
| **Passed** | Shop-invalid-key-feedback revision built successfully remotely (June 20, 2026). |
| **Passed** | Yorn-invalid-key-feedback revision built successfully remotely (June 20, 2026). |
| **Passed** | Yorn-invalid-key-text revision built successfully remotely (June 20, 2026). |
| **Passed** | Quit-cancel-feedback revision built successfully remotely (June 20, 2026). |
| **Passed** | Overworld-invalid-key-feedback revision built and validated successfully remotely (June 20, 2026). |
| **Passed** | Field-guide-return-text revision built successfully remotely (June 20, 2026). |
| **Passed** | Potion-refusal-sfx revision built successfully remotely (June 20, 2026). |
| **Passed** | Guard-sfx revision built successfully remotely (June 20, 2026). |
| **Passed** | Contextual-use-refusal-sfx revision built successfully remotely (June 20, 2026). |
| **Passed** | Town-talk-sfx revision built successfully remotely (June 20, 2026). |
| **Passed** | Castle-talk-sfx revision built successfully remotely (June 20, 2026). |
| **Passed** | Waystone-reread-sfx revision built successfully remotely (June 20, 2026). |
| **Passed** | Reed-ford-sfx revision built successfully remotely (June 20, 2026). |
| **Passed** | Shop-exit-sfx revision built successfully remotely (June 20, 2026). |
| **Passed** | Thief-steal-sfx revision built successfully remotely (June 20, 2026). |
| **Passed** | Troll-regen-sfx revision built successfully remotely (June 20, 2026). |
| **Passed** | Skeleton-wake-sfx revision built successfully remotely (June 20, 2026). |
| **Passed** | Guarded-hit-sfx revision built successfully remotely (June 20, 2026). |
| **Passed** | Calm-water-variant revision built successfully remotely (June 20, 2026). |
| **Passed** | Field-guide-open-sfx revision built successfully remotely (June 20, 2026). |
| **Passed** | Potion-panel-label revision built successfully remotely (June 20, 2026). |
| **Passed** | Treasure-reward-sfx revision built successfully remotely (June 20, 2026). |
| **Pending** | Build and validate the current overworld-route-sightline revision remotely. |
| **Passed** | Launch from DOS/65, view title, dismiss title, and reach the overworld. |
| **Passed** | Launch twice with noticeably different title-screen wait times; confirm initial monster placement differs. |
| **Passed** | Confirm both a rapid title keypress and a long wait begin normally without a frozen or repeating RNG. |
| **Pending** | Using the same title key after different delays produces different encounter or damage sequences. |
| **Passed** | Quit with `Q`, confirm both Yes and No paths, and verify a clean DOS/65 return. |
| **Passed** | At the quit confirmation prompt, invalid keys play the blocked cue, show `Press Y or N.`, and keep waiting for `Y` or `N`. |
| **Passed** | At the quit confirmation prompt, pressing `N` returns to the game and shows `Adventure continues.` |
| **Passed** | Press an invalid overworld command key; confirm it plays the blocked cue, shows `Press ? for help.`, and does not consume a monster/status turn. |
| **Passed** | Press `T` where no contextual action is available in the overworld, castle, and non-interactive locations; confirm it plays the blocked cue, shows the proper message, and does not consume a turn. |

## Quest And Locations

| Status | Test |
|--------|------|
| **Passed** | Receive the Wyrm Key quest from King Aldren. |
| **Passed** | Defeat the Wyrm Warden, receive the key, and return to Aldren to open the lair. |
| **Pending** | Enter and leave Eastmere, Valehaven, the castle, Sunken Shrine, and dragon dungeon. |
| **Pending** | From the new-game start, the northern road visibly connects to the castle approach and Eastmere route without requiring cross-country wandering. |
| **Pending** | From Valehaven or the old waystone, the broken southern road visibly leads toward the Sunken Shrine while the final approach still feels marshy and dangerous. |
| **Pending** | Defeat the dragon, see the return-to-Aldren objective, and trigger the ending at the castle. |
| **Pending** | Verify chest rewards can grant gold, provisions, healing, lore XP, poison cure/full heal, a stored potion, and an equipment upgrade, each with a reward cue. |
| **Pending** | Verify shop purchases, healing, provisions, and equipment upgrades still work after receiving chest rewards. |
| **Pending** | While the shop menu remains open, gold, food, equipment, HP, poison, and potion changes update on the right-side panel after each purchase. |
| **Pending** | Buy healing potions from Eastmere and Valehaven; confirm Valehaven is cheaper, gold decreases, potion count rises, and the side panel redraws cleanly. |
| **Pending** | Try buying a potion with a full pouch and with insufficient gold; confirm the shop shows the proper refusal and does not change gold or potion count. |
| **Pending** | Press an invalid key in the shop menu; confirm it plays the blocked cue, shows `Choose 1-5, or X to leave.`, and keeps the menu open. |
| **Passed** | Leave a shop with `X`; confirm it plays the door cue, restores the map and panel, and shows the farewell message. |
| **Pending** | Shop purchases play a reward cue, while unaffordable, full-pouch, and max-equipment refusals play the blocked cue without making the menu feel sluggish. |
| **Pending** | Pressing `P` with a stored potion heals up to 20 HP, cures poison, decrements the potion count, and consumes a monster/status turn. |
| **Pending** | Pressing `P` with no potion, or while fully healthy and unpoisoned, plays the blocked cue, shows a message, and does not consume a turn. |
| **Passed** | Shop panel shows current weapon and armor names plus their actual trait text after every purchase. |
| **Pending** | Pressing `T` away from shop counters in Eastmere and Valehaven plays a soft talk cue and shows town-specific rumor text for each quest state. |
| **Pending** | Castle and town non-shop interaction messages play appropriate talk/refusal cues, read naturally, and fit the message log. |
| **Passed** | Eastmere uses its dense coastal map and displays `Eastmere` in the panel. |
| **Passed** | Valehaven uses its open canal map and displays `Valehaven` in the panel. |
| **Passed** | Eastmere and Valehaven use visibly distinct palettes while keeping floor, wall, water, shop counter, exit, and player contrast clear. |
| **Passed** | Eastmere sells cheaper equipment; Valehaven sells cheaper healing, more provisions, and cheaper potions. |

## Regions And Encounters

| Status | Test |
|--------|------|
| **Passed** | `Where` shows `Northreach` in rows 0-20, `Wyrmhold Vale` in rows 21-43, and `Sunken March` in rows 44-63. |
| **Passed** | Crossing rows 20/21 and 43/44 updates the displayed region without visual corruption. |
| **Pending** | Northreach favors orcs and skeletons; Wyrmhold Vale is mixed; Sunken March favors snakes, thieves, and trolls. |
| **Pending** | After Aldren opens the dragon's lair, newly spawned overworld groups feel noticeably tougher. |
| **Passed** | Entering and leaving an interior restores surviving overworld monsters at the same coordinates, with their previous health and behavior state. |
| **Pending** | Returning from an interior only adds reinforcements when fewer than three overworld monsters remain. |
| **Pending** | Repeatedly entering and leaving locations does not create a fresh full encounter group each time. |

## Terrain Identity

| Status | Test |
|--------|------|
| **Passed** | Roads and bridges display `Road: saves ration` and do not consume the normal travel ration. |
| **Passed** | Forests display `Forest cover`; monsters more than five cells away stop pursuing until the player leaves cover or comes closer. |
| **Passed** | Hills display `High ground +2` and add exactly two damage to player attacks. |
| **Pending** | Marshes display `Marsh: costly`, consume two base rations, and can poison the player. |
| **Pending** | Leather reduces marsh poison frequency; plate adds its extra ration cost on every terrain. |
| **Pending** | Poison status overrides terrain text, shows the remaining poison count, and returns to terrain/healthy text when cured or expired. |
| **Pending** | At 10 HP or fewer, the status panel shows `HP low`; above that, normal food/terrain/healthy text returns. |
| **Pending** | At 25 provisions or fewer, the status panel shows `Food low`; above that, normal terrain/healthy text returns. |
| **Pending** | Poison, `HP low`, and `Food low` status warnings use a visibly stronger warning color than normal terrain and healthy text. |

## Handcrafted Discoveries

| Status | Test |
|--------|------|
| **Pending** | Reaching the northern forest cache at `(30,4)` awards 40 gold and 75 provisions once. |
| **Pending** | Reaching the hilltop cairn at `(31,26)` grants five maximum health, fully heals, and cures poison once. |
| **Pending** | Reaching the old waystone at `(29,40)` displays its route clue once; pressing `T` there plays the talk cue and displays it again. |
| **Pending** | Pressing `T` at the reed ford endpoints `(20,57)` and `(32,59)` plays the ford cue, crosses to the other side, costs 6 HP and up to 20 provisions, and consumes a turn. |
| **Pending** | The reed ford refuses crossing at 6 HP or less and refuses if a monster occupies the destination. |
| **Pending** | Discovery rewards persist after entering and leaving interiors and do not trigger a second time. |
| **Pending** | Discovery and chest reward messages/sounds are noticeable without disrupting the turn loop. |

## Combat And Monster Identity

| Status | Test |
|--------|------|
| **Pending** | Snake surges, can poison, and leather armor sometimes resists venom. |
| **Pending** | Skeleton guards until approached and wakes with its own cue; thief steals with its own cue then flees; troll moves slowly and regenerates with its own cue. |
| **Pending** | `G` guard plays its brace cue, consumes a turn, guarded melee impacts use their own cue, and guard reduces melee damage without blocking theft, poison, or dragon breath. |
| **Pending** | Dagger critical hits, sword reliability, and axe damage variance are perceptibly distinct. |
| **Pending** | Plate armor consumes an extra provision per successful step. |
| **Pending** | Dragon breath warning lane renders clearly, can be dodged, deals damage, and clears correctly. |

## Presentation And Regression

| Status | Test |
|--------|------|
| **Pending** | Press `?` from the overworld and every interior; the field guide opens with a soft cue, does not consume a turn, and closes on the next key. |
| **Pending** | The field guide border, controls, terrain hints, status-warning note, shop/supply note, current objective, and return-to-map prompt fit cleanly with no stale characters. |
| **Passed** | Field guide shows `Version 0.9-dev` near the bottom without crowding the return prompt. |
| **Passed** | Closing the field guide fully restores the viewport, panel, monsters, breath warning, and message log. |
| **Passed** | Player, monsters, terrain, landmarks, and breath glyphs render in the correct order and orientation. |
| **Passed** | Title screen shows `Version 0.9-dev` under the summons line without stale characters or layout crowding. |
| **Retest** | Water and town canal texture remains calm during movement while still avoiding an obvious repeated grid. |
| **Passed** | Bridge, floor, door, road, and mountain-variant art still reads clearly after duplicate-pattern cleanup. |
| **Passed** | Panel values, region names, objective text, status text, messages, and shop text fit without stale characters. |
| **Pending** | Side panel displays the `Potions:` count cleanly as it changes. |
| **Passed** | Attack, hurt, guard, guarded impact, talk, field guide, skeleton wake, thief steal, troll regeneration, critical, poison, door, treasure, quest, victory, and defeat sounds are distinct and not excessively slow. |
| **Pending** | Victory screen shows the Champion line, closing narrative, DOS/65/6502PC credits, and prompt without stale characters. |
| **Passed** | Defeat screen shows the wound/starvation cause, closing narrative, and prompt without stale characters. |
| **Passed** | Death by wounds and starvation both show the correct defeat cause line. |
| **Pending** | Complete a new-game-to-victory playthrough without a dead end or crash. |

## Balance Notes

Record playthrough observations here before changing values.

| Date | Build | Area/System | Observation | Follow-up |
|------|-------|-------------|-------------|-----------|
| | | | | |
