//! A deterministic, continuous world built from small, hand-shaped biomes.
//!
//! Positions are world pixels and refer to the bottom center of a sprite. The
//! collision footprints deliberately cover trunks and foundations, not canopies.
use wgame::glam::Vec2;

pub const TILE: f32 = 16.0;
pub const WIDTH: usize = 160;
pub const HEIGHT: usize = 128;
pub const PLAYER_RADIUS: f32 = 5.0;
pub const BRIARGLEN: Vec2 = Vec2::new(680.0, 1000.0);
pub const WILLOWFORD: Vec2 = Vec2::new(1850.0, 900.0);
pub const START: Vec2 = Vec2::new(680.0, 1016.0);
pub const BRIDGE_Y: f32 = 960.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    Grass,
    Forest,
    Path,
    Water,
    Sand,
    Bridge,
    Farmland,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropKind {
    Tree,
    Pine,
    House,
    Well,
    Sign,
    Rock,
    Flowers,
    Fence,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Prop {
    pub kind: PropKind,
    pub pos: Vec2,
    pub variant: u8,
}

#[derive(Clone, Debug)]
pub struct Npc {
    pub name: &'static str,
    pub role: &'static str,
    pub pos: Vec2,
    pub home: Vec2,
    pub variant: u8,
    /// 0 down, 1 left, 2 right, 3 up.
    pub facing: u8,
    pub walking: bool,
    target: Vec2,
    next_walk: f32,
    walks: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Quest {
    #[default]
    NotStarted,
    Carrying,
    Delivered,
}

impl Quest {
    pub fn objective(self) -> &'static str {
        match self {
            Self::NotStarted => "Speak to Mira in Briarglen",
            Self::Carrying => "Bring Mira's parcel to Rowan in Willowford",
            Self::Delivered => "Parcel delivered · the vale is yours to explore",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Dialogue {
    pub speaker: &'static str,
    pub title: &'static str,
    pub text: &'static str,
}

#[derive(Clone, Debug)]
pub struct Player {
    pub pos: Vec2,
    pub facing: u8,
    pub walking: bool,
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}

impl Player {
    pub fn new() -> Self {
        Self {
            pos: START,
            facing: 0,
            walking: false,
        }
    }
}

pub struct World {
    pub tiles: Vec<Tile>,
    pub props: Vec<Prop>,
    pub npcs: Vec<Npc>,
}

/// The Mere is one uninterrupted north-to-south river, with one bridge.
pub fn river_x(y: f32) -> f32 {
    1280.0 + (y * 0.0045).sin() * 110.0 + (y * 0.012).sin() * 24.0
}

fn river_width(y: f32) -> f32 {
    38.0 + (y * 0.006 + 1.2).sin() * 7.0
}

fn road() -> [Vec2; 8] {
    [
        BRIARGLEN,
        Vec2::new(835.0, 1030.0),
        Vec2::new(1000.0, 980.0),
        Vec2::new(river_x(BRIDGE_Y) - 90.0, BRIDGE_Y),
        Vec2::new(river_x(BRIDGE_Y) + 90.0, BRIDGE_Y),
        Vec2::new(1420.0, 944.0),
        Vec2::new(1630.0, 898.0),
        WILLOWFORD,
    ]
}

fn segment_distance(point: Vec2, a: Vec2, b: Vec2) -> f32 {
    let line = b - a;
    let t = ((point - a).dot(line) / line.length_squared().max(0.001)).clamp(0.0, 1.0);
    point.distance(a + line * t)
}

fn road_distance(pos: Vec2) -> f32 {
    road()
        .windows(2)
        .map(|pair| segment_distance(pos, pair[0], pair[1]))
        .fold(f32::MAX, f32::min)
}

fn village_distance(pos: Vec2) -> f32 {
    pos.distance(BRIARGLEN).min(pos.distance(WILLOWFORD))
}

fn forest_at(pos: Vec2) -> bool {
    let woods = [
        (Vec2::new(395.0, 340.0), Vec2::new(590.0, 460.0)),
        (Vec2::new(1990.0, 300.0), Vec2::new(585.0, 480.0)),
        (Vec2::new(345.0, 1710.0), Vec2::new(485.0, 475.0)),
        (Vec2::new(2020.0, 1670.0), Vec2::new(580.0, 520.0)),
    ];
    let ripple = (pos.x * 0.019).sin() * 0.1 + (pos.y * 0.013).cos() * 0.13;
    woods
        .iter()
        .any(|(center, extent)| ((pos - *center) / *extent).length() < 0.93 + ripple)
}

// Stable integer mixing means map details never depend on wall-clock time or
// platform RNG implementation, and saved positions always refer to the same map.
fn hash(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^ (x >> 16)
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    pub fn new() -> Self {
        let mut world = Self {
            tiles: vec![Tile::Grass; WIDTH * HEIGHT],
            props: Vec::new(),
            npcs: Vec::new(),
        };

        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let pos = Vec2::new((x as f32 + 0.5) * TILE, (y as f32 + 0.5) * TILE);
                let distance_to_river = (pos.x - river_x(pos.y)).abs();
                let half_width = river_width(pos.y);
                let terrain = if (pos.y - BRIDGE_Y).abs() <= 24.0
                    && (pos.x - river_x(BRIDGE_Y)).abs() < 90.0
                {
                    Tile::Bridge
                } else if distance_to_river < half_width {
                    Tile::Water
                } else if distance_to_river < half_width + 16.0 {
                    Tile::Sand
                } else if road_distance(pos) < 21.0 {
                    Tile::Path
                } else if village_distance(pos) > 210.0 && forest_at(pos) {
                    Tile::Forest
                } else {
                    Tile::Grass
                };
                world.tiles[y * WIDTH + x] = terrain;
            }
        }

        world.build_village(BRIARGLEN, 0);
        world.build_village(WILLOWFORD, 1);
        world.plant_landscape();
        world.populate();
        world
    }

    pub fn tile(&self, x: usize, y: usize) -> Tile {
        if x < WIDTH && y < HEIGHT {
            self.tiles[y * WIDTH + x]
        } else {
            Tile::Water
        }
    }

    fn paint_path(&mut self, from: Vec2, to: Vec2, width: f32) {
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let pos = Vec2::new((x as f32 + 0.5) * TILE, (y as f32 + 0.5) * TILE);
                if segment_distance(pos, from, to) < width && self.tile(x, y) != Tile::Water {
                    self.tiles[y * WIDTH + x] = Tile::Path;
                }
            }
        }
    }

    fn build_village(&mut self, center: Vec2, village: u8) {
        let homes = [
            Vec2::new(-143.0, -80.0),
            Vec2::new(127.0, -98.0),
            Vec2::new(-132.0, 126.0),
            Vec2::new(145.0, 138.0),
        ];
        for (index, offset) in homes.into_iter().enumerate() {
            let pos = center + offset;
            self.paint_path(center, pos + Vec2::new(0.0, 14.0), 10.5);
            self.props.push(Prop {
                kind: PropKind::House,
                pos,
                variant: (index as u8 + village) % 4,
            });
            for side in [-1.0, 1.0] {
                self.props.push(Prop {
                    kind: PropKind::Flowers,
                    pos: pos + Vec2::new(side * 35.0, -5.0),
                    variant: (index as u8 + village) % 3,
                });
            }
        }
        self.props.push(Prop {
            kind: PropKind::Well,
            pos: center + Vec2::new(-52.0, 4.0),
            variant: village,
        });
        self.props.push(Prop {
            kind: PropKind::Sign,
            pos: center + Vec2::new(75.0, 49.0),
            variant: village,
        });

        // Small open farm plots at each village's edge, with a gap in the fence.
        let farm = center + Vec2::new(-220.0, 125.0);
        let x0 = (farm.x / TILE) as usize;
        let y0 = (farm.y / TILE) as usize;
        for y in y0..y0 + 4 {
            for x in x0..x0 + 5 {
                self.tiles[y * WIDTH + x] = Tile::Farmland;
            }
        }
        for x in [0.0, 32.0, 64.0] {
            self.props.push(Prop {
                kind: PropKind::Fence,
                pos: farm + Vec2::new(x + 8.0, -5.0),
                variant: 0,
            });
        }
        self.props.push(Prop {
            kind: PropKind::Tree,
            pos: center + Vec2::new(-37.0, -110.0),
            variant: 0,
        });
        self.props.push(Prop {
            kind: PropKind::Tree,
            pos: center + Vec2::new(34.0, 156.0),
            variant: 1,
        });

        // Small groves frame the village green without hiding the cottages or
        // narrowing the road. Each group leaves a walkable gap to its neighbors.
        for (index, offset) in [
            Vec2::new(-205.0, -140.0),
            Vec2::new(-232.0, -100.0),
            Vec2::new(-198.0, -70.0),
            Vec2::new(205.0, -142.0),
            Vec2::new(235.0, -105.0),
            Vec2::new(-212.0, 68.0),
            Vec2::new(-242.0, 32.0),
            Vec2::new(210.0, 72.0),
            Vec2::new(239.0, 109.0),
        ]
        .into_iter()
        .enumerate()
        {
            self.props.push(Prop {
                kind: if index == 3 || index == 6 {
                    PropKind::Pine
                } else {
                    PropKind::Tree
                },
                pos: center + offset,
                variant: (index as u8 + village) % 3,
            });
        }
        for (index, offset) in [
            Vec2::new(-85.0, -5.0),
            Vec2::new(-70.0, -75.0),
            Vec2::new(-18.0, 70.0),
            Vec2::new(45.0, -55.0),
            Vec2::new(100.0, 55.0),
            Vec2::new(68.0, 125.0),
        ]
        .into_iter()
        .enumerate()
        {
            self.props.push(Prop {
                kind: PropKind::Flowers,
                pos: center + offset,
                variant: (index as u8 + village) % 3,
            });
        }
        for (index, offset) in [Vec2::new(-95.0, 45.0), Vec2::new(40.0, -70.0)]
            .into_iter()
            .enumerate()
        {
            self.props.push(Prop {
                kind: PropKind::Rock,
                pos: center + offset,
                variant: index as u8,
            });
        }
    }

    fn plant_landscape(&mut self) {
        // Offset grid keeps trunks readable and allows room to walk through woods.
        for y in (2..HEIGHT - 2).step_by(3) {
            for x in (2..WIDTH - 2).step_by(3) {
                let seed = hash((y * WIDTH + x) as u32 + 47);
                let pos = Vec2::new(
                    x as f32 * TILE + 8.0 + ((seed >> 8) % 21) as f32 - 10.0,
                    y as f32 * TILE + 8.0 + ((seed >> 16) % 21) as f32 - 10.0,
                );
                if village_distance(pos) < 246.0
                    || road_distance(pos) < 54.0
                    || (pos.x - river_x(pos.y)).abs() < river_width(pos.y) + 32.0
                {
                    continue;
                }
                let forest = forest_at(pos);
                if (forest && seed % 100 < 77) || (!forest && seed % 100 < 7) {
                    self.props.push(Prop {
                        kind: if (seed >> 4).is_multiple_of(4) {
                            PropKind::Pine
                        } else {
                            PropKind::Tree
                        },
                        pos,
                        variant: ((seed >> 20) % 3) as u8,
                    });
                } else if seed.is_multiple_of(31) {
                    self.props.push(Prop {
                        kind: PropKind::Rock,
                        pos,
                        variant: ((seed >> 20) % 3) as u8,
                    });
                } else if seed.is_multiple_of(7) && !forest {
                    self.props.push(Prop {
                        kind: PropKind::Flowers,
                        pos,
                        variant: ((seed >> 20) % 3) as u8,
                    });
                }
            }
        }
        // A weathered ring in the northern meadow rewards a little wandering.
        for (index, offset) in [
            Vec2::new(-25.0, -10.0),
            Vec2::new(4.0, -23.0),
            Vec2::new(30.0, 0.0),
            Vec2::new(12.0, 25.0),
            Vec2::new(-20.0, 23.0),
        ]
        .into_iter()
        .enumerate()
        {
            self.props.push(Prop {
                kind: PropKind::Rock,
                pos: Vec2::new(928.0, 450.0) + offset,
                variant: (index % 3) as u8,
            });
        }
    }

    fn populate(&mut self) {
        let villagers = [
            ("Mira", "Briarglen herbalist", Vec2::new(695.0, 1000.0), 0),
            ("Alden", "Briarglen woodcutter", Vec2::new(553.0, 962.0), 1),
            ("Pip", "Briarglen gardener", Vec2::new(568.0, 1150.0), 2),
            ("Bess", "Briarglen baker", Vec2::new(803.0, 936.0), 3),
            (
                "Rowan",
                "Willowford apothecary",
                Vec2::new(1848.0, 940.0),
                1,
            ),
            (
                "Elowen",
                "Willowford storyteller",
                Vec2::new(1700.0, 858.0),
                0,
            ),
            ("Tobin", "Willowford farmer", Vec2::new(1733.0, 1065.0), 2),
            (
                "Wren",
                "Willowford riverkeeper",
                Vec2::new(1980.0, 846.0),
                3,
            ),
        ];
        self.npcs = villagers
            .into_iter()
            .enumerate()
            .map(|(index, (name, role, pos, _variant))| Npc {
                name,
                role,
                pos,
                home: pos,
                variant: index as u8 + 1,
                facing: 0,
                walking: false,
                target: pos,
                next_walk: 4.0 + index as f32 * 1.7,
                walks: 0,
            })
            .collect();
    }

    /// Circle against terrain and the visible prop's small physical footprint.
    pub fn can_walk(&self, pos: Vec2, radius: f32) -> bool {
        if !pos.is_finite()
            || !radius.is_finite()
            || radius < 0.0
            || pos.x - radius < 0.0
            || pos.y - radius < 0.0
            || pos.x + radius >= WIDTH as f32 * TILE
            || pos.y + radius >= HEIGHT as f32 * TILE
        {
            return false;
        }
        let x0 = ((pos.x - radius) / TILE).floor() as usize;
        let x1 = ((pos.x + radius) / TILE).floor() as usize;
        let y0 = ((pos.y - radius) / TILE).floor() as usize;
        let y1 = ((pos.y + radius) / TILE).floor() as usize;
        for y in y0..=y1 {
            for x in x0..=x1 {
                if self.tile(x, y) == Tile::Water {
                    let min = Vec2::new(x as f32 * TILE, y as f32 * TILE);
                    if circle_rect(pos, radius, min, min + Vec2::splat(TILE)) {
                        return false;
                    }
                }
            }
        }
        !self.props.iter().any(|prop| match prop.kind {
            PropKind::Tree | PropKind::Pine => {
                pos.distance_squared(prop.pos + Vec2::new(0.0, -3.0)) < (radius + 5.0).powi(2)
            }
            PropKind::House => circle_rect(
                pos,
                radius,
                prop.pos + Vec2::new(-26.0, -28.0),
                prop.pos + Vec2::new(26.0, 0.0),
            ),
            PropKind::Well => circle_rect(
                pos,
                radius,
                prop.pos + Vec2::new(-10.0, -15.0),
                prop.pos + Vec2::new(10.0, 0.0),
            ),
            PropKind::Sign => circle_rect(
                pos,
                radius,
                prop.pos + Vec2::new(-3.0, -8.0),
                prop.pos + Vec2::new(3.0, 0.0),
            ),
            PropKind::Rock => circle_rect(
                pos,
                radius,
                prop.pos + Vec2::new(-8.0, -8.0),
                prop.pos + Vec2::new(8.0, 0.0),
            ),
            PropKind::Fence => circle_rect(
                pos,
                radius,
                prop.pos + Vec2::new(-16.0, -5.0),
                prop.pos + Vec2::new(16.0, 0.0),
            ),
            PropKind::Flowers => false,
        })
    }

    /// Short steps prevent crossing a river or building during a slow frame.
    /// Splitting axes permits natural sliding along walls and river banks.
    pub fn slide_move(&self, pos: Vec2, delta: Vec2) -> Vec2 {
        if !pos.is_finite() || !delta.is_finite() {
            return pos;
        }
        let steps = (delta.length() / 3.0).ceil().max(1.0) as usize;
        let step = delta / steps as f32;
        let mut next = pos;
        for _ in 0..steps {
            let x = next + Vec2::new(step.x, 0.0);
            if self.can_walk(x, PLAYER_RADIUS) {
                next = x;
            }
            let y = next + Vec2::new(0.0, step.y);
            if self.can_walk(y, PLAYER_RADIUS) {
                next = y;
            }
        }
        next
    }

    pub fn update(&mut self, dt: f32, elapsed: f32) {
        if !dt.is_finite() || !elapsed.is_finite() {
            return;
        }
        let dt = dt.clamp(0.0, 0.1);
        for index in 0..self.npcs.len() {
            if elapsed >= self.npcs[index].next_walk {
                let npc = &self.npcs[index];
                let seed = hash(index as u32 * 7919 + npc.walks * 131 + 89);
                let angle = (seed % 6283) as f32 * 0.001;
                let distance = 9.0 + ((seed >> 14) % 23) as f32;
                let target = npc.home + Vec2::new(angle.cos(), angle.sin()) * distance;
                let clear = self.can_walk(target, PLAYER_RADIUS);
                let npc = &mut self.npcs[index];
                if clear {
                    npc.target = target;
                }
                npc.next_walk = elapsed + 5.0 + ((seed >> 8) % 6) as f32;
                npc.walks += 1;
            }
            let old = self.npcs[index].pos;
            let delta = self.npcs[index].target - old;
            let desired = delta.normalize_or_zero() * (11.0 * dt).min(delta.length());
            let pos = self.slide_move(old, desired);
            let movement = pos - old;
            let npc = &mut self.npcs[index];
            npc.pos = pos;
            npc.walking = movement.length_squared() > 0.0001;
            if npc.walking {
                npc.facing = facing(movement);
            }
        }
    }

    pub fn region(&self, pos: Vec2) -> &'static str {
        if pos.distance(BRIARGLEN) < 230.0 {
            "Briarglen"
        } else if pos.distance(WILLOWFORD) < 230.0 {
            "Willowford"
        } else if (pos.x - river_x(pos.y)).abs() < 125.0 {
            "The River Mere"
        } else if pos.distance(Vec2::new(928.0, 450.0)) < 105.0 {
            "The Old Stones"
        } else if forest_at(pos) && pos.y < 950.0 {
            "Whisperwood"
        } else if forest_at(pos) {
            "Elderwood"
        } else {
            "Mossmeadow"
        }
    }

    pub fn interact(&self, index: usize, quest: &mut Quest) -> Dialogue {
        let Some(npc) = self.npcs.get(index) else {
            return Dialogue {
                speaker: "Moss & Mere",
                title: "A quiet moment",
                text: "The grass whispers in the breeze.",
            };
        };
        let (title, text) = match (npc.name, *quest) {
            ("Mira", Quest::NotStarted) => {
                *quest = Quest::Carrying;
                (
                    "A little kindness",
                    "A traveler! Would you bring these moonleaf herbs to Rowan in Willowford? Follow the dirt road east, over the wooden bridge. He is waiting by the village green.\n\nYou received Mira's parcel.",
                )
            }
            ("Mira", Quest::Carrying) => (
                "Across the Mere",
                "Rowan lives in Willowford, east across the river. The old wooden bridge is the only dry crossing. Follow the road, and give him my warmest wishes.",
            ),
            ("Mira", Quest::Delivered) => (
                "A welcome friend",
                "Rowan sent word that the moonleaf arrived safely. Thank you, friend. There will always be a warm hearth for you in Briarglen.",
            ),
            ("Rowan", Quest::Carrying) => {
                *quest = Quest::Delivered;
                (
                    "A kindness carried",
                    "Mira's moonleaf! Just what I needed for the village remedies. You have done Willowford a kindness.\n\nParcel delivered. You are a friend of the vale. Stay a while; the woods have stories of their own.",
                )
            }
            ("Rowan", Quest::NotStarted) => (
                "A friend across the river",
                "Welcome to Willowford. My friend Mira tends the herbs in Briarglen, across the river to the west. If you meet her, tell her Rowan sends his best.",
            ),
            ("Rowan", Quest::Delivered) => (
                "A friend of the vale",
                "The remedies are ready, thanks to you. Wander where you will, friend. The Mere has a way of bringing good people together.",
            ),
            ("Alden", _) => (
                "Whisperwood",
                "The northern forest is called Whisperwood. Leave the old trees standing and the paths will be kind to you. Watch your step around their roots.",
            ),
            ("Pip", _) => (
                "Small things growing",
                "I planted those flowers myself. One day I will grow a garden from here to Willowford! Until then, the bees seem happy enough.",
            ),
            ("Bess", _) => (
                "The morning's bread",
                "There is nothing like a walk through the meadow after sunrise. Follow the path east if you are visiting Willowford. You cannot miss the bridge.",
            ),
            ("Elowen", _) => (
                "The old stones",
                "Northwest of the bridge, five old stones stand in a meadow. My grandmother said the first folk of the vale gathered there to listen to the stars.",
            ),
            ("Tobin", _) => (
                "A quiet harvest",
                "Good earth, gentle rain, and neighbors who lend a hand. That is all a village needs. Well, that and a bridge that keeps its feet dry.",
            ),
            ("Wren", _) => (
                "One river, many stories",
                "The Mere runs all the way through our vale. The wooden bridge is the only crossing. Sometimes I stand there just to watch the water find its way.",
            ),
            _ => (
                "Welcome, traveler",
                "May your road be gentle and your hearth be warm.",
            ),
        };
        Dialogue {
            speaker: npc.name,
            title,
            text,
        }
    }
}

pub fn facing(movement: Vec2) -> u8 {
    if movement.x.abs() > movement.y.abs() {
        if movement.x < 0.0 { 1 } else { 2 }
    } else if movement.y < 0.0 {
        3
    } else {
        0
    }
}

fn circle_rect(point: Vec2, radius: f32, min: Vec2, max: Vec2) -> bool {
    let nearest = point.clamp(min, max);
    point.distance_squared(nearest) <= radius * radius
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    #[test]
    fn generation_is_repeatable_and_has_two_settlements() {
        let a = World::new();
        let b = World::new();
        assert_eq!(a.tiles, b.tiles);
        assert_eq!(a.props, b.props);
        assert_eq!(a.tiles.len(), WIDTH * HEIGHT);
        assert_eq!(
            a.props.iter().filter(|p| p.kind == PropKind::House).count(),
            8
        );
        for tile in [
            Tile::Grass,
            Tile::Forest,
            Tile::Water,
            Tile::Bridge,
            Tile::Path,
            Tile::Farmland,
        ] {
            assert!(a.tiles.contains(&tile), "missing terrain {tile:?}");
        }
        assert!(a.can_walk(START, PLAYER_RADIUS));
        assert!(a.npcs[0].pos.distance(START) < 35.0);
    }

    #[test]
    fn villages_are_reachable_on_foot() {
        let world = World::new();
        let start = ((START.x / TILE) as usize, (START.y / TILE) as usize);
        let destination = (
            (WILLOWFORD.x / TILE) as usize,
            (WILLOWFORD.y / TILE) as usize,
        );
        let mut queue = VecDeque::from([start]);
        let mut seen = vec![false; WIDTH * HEIGHT];
        seen[start.1 * WIDTH + start.0] = true;
        while let Some((x, y)) = queue.pop_front() {
            if (x, y) == destination {
                return;
            }
            for (nx, ny) in [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ] {
                if nx >= WIDTH || ny >= HEIGHT || seen[ny * WIDTH + nx] {
                    continue;
                }
                seen[ny * WIDTH + nx] = true;
                let pos = Vec2::new((nx as f32 + 0.5) * TILE, (ny as f32 + 0.5) * TILE);
                if world.can_walk(pos, PLAYER_RADIUS) {
                    queue.push_back((nx, ny));
                }
            }
        }
        panic!("Willowford is disconnected from the player's starting village");
    }

    #[test]
    fn river_is_continuous_and_bridge_is_its_only_crossing() {
        let world = World::new();
        for y in 0..HEIGHT {
            let py = (y as f32 + 0.5) * TILE;
            let pos = Vec2::new(river_x(py), py);
            if (py - BRIDGE_Y).abs() <= 24.0 {
                assert!(
                    world.can_walk(pos, PLAYER_RADIUS),
                    "bridge blocked at {pos}"
                );
            } else {
                assert!(
                    !world.can_walk(pos, PLAYER_RADIUS),
                    "river missing at {pos}"
                );
            }
        }
        let from = Vec2::new(river_x(BRIDGE_Y) - 120.0, BRIDGE_Y);
        let to = world.slide_move(from, Vec2::new(240.0, 0.0));
        assert!(to.distance(from + Vec2::new(240.0, 0.0)) < 0.01);
    }

    #[test]
    fn large_steps_cannot_tunnel_through_houses_or_the_river() {
        let world = World::new();
        let house = world
            .props
            .iter()
            .find(|p| p.kind == PropKind::House)
            .unwrap();
        let from = house.pos + Vec2::new(-60.0, -12.0);
        assert!(world.can_walk(from, PLAYER_RADIUS));
        let stopped = world.slide_move(from, Vec2::new(140.0, 0.0));
        assert!(stopped.x < house.pos.x - 26.0);
        assert!(world.can_walk(stopped, PLAYER_RADIUS));
        let from = Vec2::new(river_x(800.0) - 100.0, 800.0);
        let stopped = world.slide_move(from, Vec2::new(220.0, 0.0));
        assert!(stopped.x < river_x(800.0));
        assert!(world.can_walk(stopped, PLAYER_RADIUS));
        assert!(!world.can_walk(Vec2::new(f32::NAN, 30.0), PLAYER_RADIUS));
        assert!(!world.can_walk(Vec2::ZERO, PLAYER_RADIUS));
    }

    #[test]
    fn villagers_keep_safe_local_positions() {
        let mut world = World::new();
        for npc in &world.npcs {
            assert!(
                world.can_walk(npc.pos, PLAYER_RADIUS),
                "{} starts inside scenery",
                npc.name
            );
        }
        for frame in 0..1200 {
            world.update(0.1, frame as f32 * 0.1);
            if frame % 60 == 0 {
                for npc in &world.npcs {
                    assert!(
                        world.can_walk(npc.pos, PLAYER_RADIUS),
                        "{} walked into scenery",
                        npc.name
                    );
                    assert!(
                        npc.pos.distance(npc.home) <= 33.0,
                        "{} wandered out of the village",
                        npc.name
                    );
                }
            }
        }
    }

    #[test]
    fn parcel_quest_has_consistent_transitions() {
        let world = World::new();
        let mut quest = Quest::NotStarted;
        world.interact(4, &mut quest);
        assert_eq!(quest, Quest::NotStarted);
        world.interact(0, &mut quest);
        assert_eq!(quest, Quest::Carrying);
        world.interact(0, &mut quest);
        assert_eq!(quest, Quest::Carrying);
        world.interact(4, &mut quest);
        assert_eq!(quest, Quest::Delivered);
        world.interact(0, &mut quest);
        assert_eq!(quest, Quest::Delivered);
    }
}
