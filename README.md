# Moss & Mere

A quiet, open-world 2D fantasy RPG built in Rust with **wgame**. Wander the
Green March: sunlit plains, thick forests, two inhabited villages, and a single
winding river crossed by a wooden bridge.

The player begins in Briarglen. Speak to Mira, carry her moonleaf herbs to Rowan
in Willowford, or simply explore. Eight villagers wander near their homes and
share local stories. Far from the villages, moss slimes and briar wolves roam the
wilds. Carry an iron sword, watch for their attack windups, and retreat when you
need to recover. The map, journal, and satchel track the journey.

## Play

A prebuilt Linux executable is included locally for immediate play:

```sh
cd ~/develop/my/moss-and-mere
./bin/moss-and-mere
```

To build from source after making changes:

Requires a current Rust toolchain, a desktop graphics driver, and the neighboring
`../wgame` checkout. This project references `../wgame/wgame` in `Cargo.toml`.

```sh
cd ~/develop/my/moss-and-mere
cargo run --locked
```

| Control | Action |
| --- | --- |
| WASD / arrow keys | Walk |
| Shift | Run |
| Space / left click | Swing the sword; click toward a target to aim |
| E / Enter | Talk to a nearby villager; close dialogue |
| M | World map |
| J / Tab | Journal and satchel |
| H | Help |
| + / − / mouse wheel | Zoom |
| Escape | Close a panel or pause |
| Q, while paused | Save and leave |

The toolbar and panel close buttons also support the mouse. The game fits its
view into resized windows. Trees, buildings, rocks, fences, and water block
movement; the bridge connects the two banks. Buildings are exterior landmarks.

Progress saves every fifteen seconds, on quest milestones and discoveries, and
when exiting normally. Native saves live at
`${XDG_DATA_HOME:-$HOME/.local/share}/moss-and-mere/save.txt`.
Run `cargo run --locked -- --fresh` to begin again; playing a fresh journey
replaces the previous save. There is one save slot. Invalid saves are backed up before replacement.

## Combat and village reactions

Space strikes in the direction you face; clicking the landscape aims a strike.
The sword has a short cooldown and only hits nearby targets in front of you.
Orange rings warn of an incoming bite or punch: step away before it lands. The
health bar, enemy bars, flashes, and floating numbers show damage.

Monsters live at least 400 world pixels from the village centers. They patrol,
pursue nearby travellers, and return to their territories. Village wards keep
monsters outside, and peaceful villages gradually restore your health.

**Striking a resident turns their entire village hostile.** Everyone in that
village joins the fight, refuses conversation, and stops offering sanctuary.
The other village remains peaceful. Leave the offended village's area for a
minute to let tempers cool. Residents knocked down in a fight recover after
30 seconds, keeping story characters available. If you are defeated, you are
rescued to a discovered village with your belongings and quest progress intact;
the fighting ends and your health is restored.

Health, monster defeat count, and village hostility are saved. Earlier saves
load with full health and peaceful villages. Monsters return after 90 seconds
once you leave their immediate area, and encounters reset when loading a game.

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
cargo run --locked -- --smoke
```

These preview commands start a fresh world, render twelve frames, and exit
without saving. They require a graphical display. For a static browser build,
use `trunk build --no-default-features --features web`.

## Implementation

- `src/world.rs`: deterministic terrain, collision, villagers, and conversations.
- `src/game.rs`: player movement, discovery, quest, recovery, camera, and pause state.
- `src/combat.rs`: monster habitats, combat, village retaliation, and respawning.
- `src/art.rs`: original pixel-art terrain, cottages, trees, and animated people.
- `src/render.rs`: wgame sprite rendering, map, journal, dialogue, and HUD.
- `src/save.rs`: validated versioned saves with atomic native replacement.

The 160 × 128 tile landscape is continuous. Sprites use nearest sampling and
are sorted by their feet so the player can pass behind trees and cottages.
Original art is generated at startup; no network or downloaded images are
required. Embedded DejaVu fonts are covered by `assets/FONT-LICENSE.txt`.

This first chapter focuses on travelling, village life, and wilderness combat.
Interiors, trading, and a larger quest campaign are outside this version.

```sh
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

## License

MIT, except the separately licensed embedded fonts.
