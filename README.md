# Moss & Mere

A quiet, open-world 2D fantasy RPG built in Rust with **wgame**. Wander the
Green March: sunlit plains, thick forests, two inhabited villages, and a single
winding river crossed by a wooden bridge.

The player begins in Briarglen. Speak to Mira, carry her moonleaf herbs to Rowan
in Willowford, or simply explore. Eight villagers wander near their homes and
share local stories. Far from the villages, moss slimes and briar wolves roam the
wilds. Carry an iron sword, watch for their attack windups, and retreat when you
need to recover. Hunt for coins and iron shards, improve your equipment at
Alden's forge, and explore the ruined halls of Emberwatch. The map, journal, and
satchel track the journey.

## Play

A prebuilt Linux executable is included locally for immediate play:

```sh
cd ~/develop/my/moss-and-mere
./bin/moss-and-mere
```

To build from source after making changes:

Requires a current Rust toolchain, a desktop graphics driver, SDL2 development
libraries for native sound, and the neighboring `../wgame` checkout. This project
references `../wgame/wgame` in `Cargo.toml`. On Debian or Ubuntu, install the audio
build dependency with `sudo apt install libsdl2-dev`. The local Linux executable
uses the system SDL2 runtime.

```sh
cd ~/develop/my/moss-and-mere
cargo run --locked
```

| Control | Action |
| --- | --- |
| WASD / arrow keys | Walk |
| Shift | Run; consumes stamina |
| Space / left click | Swing the sword; click toward a target to aim |
| Q / right click | Dodge; consumes stamina |
| E / Enter | Talk, examine ruins, open a cache, take a relic, or close dialogue |
| 1 / 2, at the forge | Upgrade sword / armor |
| R | Open the nearby village's council and reputation panel |
| 1, at the council | Pay reparations when you owe them |
| M | World map |
| J / Tab | Journal and satchel |
| H | Help |
| U | Mute / unmute sound |
| + / − / mouse wheel | Zoom |
| Escape | Close a panel or pause |
| Q, while paused | Save and leave |

The toolbar and panel close buttons also support the mouse. The game fits its
view into resized windows. Trees, buildings, rocks, fences, and water block
movement; the bridge connects the two banks. Buildings are exterior landmarks.
Talking to Alden and closing his dialogue opens the forge menu. Buy an upgrade
with its number key; each purchase equips it immediately.

Progress saves every fifteen seconds, on quest milestones and discoveries, and
when exiting normally. Native saves live at
`${XDG_DATA_HOME:-$HOME/.local/share}/moss-and-mere/save.txt`.
Run `cargo run --locked -- --fresh` to begin again; playing a fresh journey
replaces the previous save. There is one save slot. Invalid saves are backed up before replacement.

## Combat and village reactions

Space strikes in the direction you face; clicking the landscape aims a strike.
The sword has a short cooldown and only hits nearby targets in front of you.
Orange rings warn of an incoming bite or punch: step away before it lands. The
health bar, enemy bars, flashes, and floating numbers show damage. Dodge through
danger with Q or right click. Sprinting, dodging, and sword swings draw from your
stamina, which recovers when you give it a moment.

Monsters live at least 400 world pixels from the village centers. They patrol,
pursue nearby travellers, and return to their territories. Village wards keep
monsters outside, and peaceful villages gradually restore your health.

**Striking a resident turns their entire village hostile.** Everyone in that
village joins the fight, refuses conversation, and stops offering sanctuary.
Hurting residents also lowers that village's lasting reputation. The other
village remains peaceful. Leave the offended village's area for a minute to let
tempers cool; your reputation still needs repair. Open its council with R and
pay reparations to restore peace and a neutral reputation. Residents knocked
down in a fight recover after 30 seconds, keeping story characters available.
If you are defeated, you are rescued to a discovered village with your belongings
and quest progress intact; the fighting ends and your health is restored.

Health, monster defeat count, village hostility, reputation, equipment, collected loot,
and ruin discoveries are saved. Save version 3 migrates the earlier version 1
and 2 formats with sensible defaults for new progress. Ordinary monsters return
after 90 seconds once you leave their immediate area, and their encounters reset
when loading a game. The Emberwatch guardian stays defeated once beaten.

## The Emberheart chapter

Alden is Briarglen's blacksmith, beside the anvil near its northwestern cottage.
Speak with him to learn about the Emberheart, an heirloom lost in Emberwatch
Ruins. Cross the bridge to Willowford, then explore the forest to the northeast.
The ruin has a southern entrance, a guardian's hall, and a hidden treasury. Watch
for an engraved stone, and use E when an interaction prompt appears.

Defeated creatures leave coins and iron shards. Walk over their spoils to collect
them, then bring these supplies to Alden for stronger swords and armor. The
hidden cache offers another source of supplies. Completing Mira's delivery also
earns coins and goodwill in both villages. A Briarglen reputation of 30 or more
earns a 20% coin discount at the forge; materials still cost the same.

Defeat the guardian, take the Emberheart from the northern pedestal, and return
to Alden. He rewards you with **Emberbrand**, a unique sword, 45 coins, and
Briarglen's trust. Quest rewards and the hidden cache can each be claimed once.

The journey now has changing daylight and rain, with footsteps, sword impacts,
birds, river ambience, and weather sounds. Audio is generated locally. Press U
to mute it.

## Browser build

The same game supports WebGL2 and saves to browser local storage. Install the
Wasm target and Trunk, then run:

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
trunk serve --no-default-features --features web
```

Use a keyboard-equipped browser. Native screenshots are available for reviewing
the actual wgame renderer:

```sh
cargo run --locked -- --screenshot /tmp/moss-and-mere.png
cargo run --locked -- --map --screenshot /tmp/moss-and-mere-map.png
cargo run --locked -- --view river --screenshot /tmp/moss-and-mere-river.png
cargo run --locked -- --view combat --screenshot /tmp/moss-and-mere-combat.png
cargo run --locked -- --view ruins --screenshot /tmp/moss-and-mere-ruins.png
cargo run --locked -- --view forge --screenshot /tmp/moss-and-mere-forge.png
cargo run --locked -- --view rain --screenshot /tmp/moss-and-mere-rain.png
cargo run --locked -- --smoke
```

These preview commands start a fresh world, render twelve frames, and exit
without saving. They require a graphical display. For a static browser build,
use `trunk build --no-default-features --features web`.

## Implementation

- `src/world.rs`: deterministic terrain, collision, villagers, and conversations.
- `src/game.rs`: player movement, discovery, quest, recovery, camera, and pause state.
- `src/combat.rs`: monster habitats, combat, village retaliation, and respawning.
- `src/progress.rs`: equipment, forge purchases, reputation, and the Emberheart quest.
- `src/art.rs`: original pixel-art terrain, cottages, trees, and animated people.
- `src/render.rs`: wgame sprite rendering, map, journal, dialogue, and HUD.
- `src/audio.rs`: synthesized effects and environmental ambience.
- `src/save.rs`: validated versioned saves with atomic native replacement.

The 160 × 128 tile landscape is continuous. Sprites use nearest sampling and
are sorted by their feet so the player can pass behind trees and cottages.
Original art is generated at startup; no network or downloaded images are
required. Embedded DejaVu fonts are covered by `assets/FONT-LICENSE.txt`.

The chapter includes travelling, village life, wilderness combat, equipment
crafting, and an outdoor dungeon. Cottage interiors and a larger quest campaign
remain outside this version.

```sh
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

## License

MIT, except the separately licensed embedded fonts.
