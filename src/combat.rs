//! Wilderness encounters. Village wards and world collision apply to every AI
//! movement, knockback, and attack ray, including during slow frames.
use crate::world::{BRIARGLEN, GUARDIAN_HOME, PLAYER_RADIUS, Player, WILLOWFORD, World};
use wgame::glam::Vec2;

pub const MAX_HEALTH: f32 = 100.0;
pub const MAX_STAMINA: f32 = 100.0;
pub const DODGE_DURATION: f32 = 0.22;
pub const SAFE_RADIUS: f32 = 320.0;
const HOME_LEASH: f32 = 180.0;
pub const RESPAWN_TIME: f32 = 90.0;
pub const SWING_DURATION: f32 = 0.22;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Slime,
    Wolf,
    Guardian,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Slime => "Moss slime",
            Self::Wolf => "Briar wolf",
            Self::Guardian => "Thorn Guardian",
        }
    }

    pub fn max_health(self) -> f32 {
        match self {
            Self::Slime => 40.0,
            Self::Wolf => 65.0,
            Self::Guardian => 180.0,
        }
    }

    pub fn windup_duration(self) -> f32 {
        if self == Self::Guardian { 0.7 } else { 0.4 }
    }

    fn reach(self) -> f32 {
        if self == Self::Guardian { 48.0 } else { 30.0 }
    }

    fn speed(self) -> f32 {
        match self {
            Self::Slime => 34.0,
            Self::Wolf => 53.0,
            Self::Guardian => 42.0,
        }
    }

    fn damage(self) -> f32 {
        match self {
            Self::Slime => 12.0,
            Self::Wolf => 18.0,
            Self::Guardian => 22.0,
        }
    }
}

pub struct Monster {
    pub kind: Kind,
    pub pos: Vec2,
    pub home: Vec2,
    pub health: f32,
    pub facing: u8,
    pub walking: bool,
    pub hit_flash: f32,
    pub windup: f32,
    pub respawn: f32,
    bite_cooldown: f32,
    bite_dir: Vec2,
}

impl Monster {
    fn new(kind: Kind, pos: Vec2) -> Self {
        Self {
            kind,
            pos,
            home: pos,
            health: kind.max_health(),
            facing: 0,
            walking: false,
            hit_flash: 0.0,
            windup: 0.0,
            respawn: 0.0,
            bite_cooldown: 0.0,
            bite_dir: Vec2::Y,
        }
    }
}

pub struct LootDrop {
    pub pos: Vec2,
    pub coins: u32,
    pub materials: u32,
    pub life: f32,
}

pub struct HitEffect {
    pub pos: Vec2,
    pub amount: u32,
    pub life: f32,
    pub player: bool,
}

pub const RESIDENT_HEALTH: f32 = 60.0;

pub struct Resident {
    pub health: f32,
    pub hit_flash: f32,
    pub windup: f32,
    pub down: f32,
    cooldown: f32,
    bite_dir: Vec2,
}

impl Resident {
    fn new() -> Self {
        Self {
            health: RESIDENT_HEALTH,
            hit_flash: 0.0,
            windup: 0.0,
            down: 0.0,
            cooldown: 0.0,
            bite_dir: Vec2::Y,
        }
    }
}

fn village(home: Vec2) -> usize {
    usize::from(home.distance_squared(WILLOWFORD) < home.distance_squared(BRIARGLEN))
}

fn village_center(index: usize) -> Vec2 {
    if index == 0 { BRIARGLEN } else { WILLOWFORD }
}

pub struct Combat {
    pub monsters: Vec<Monster>,
    pub hostility: [f32; 2],
    pub residents: Vec<Resident>,
    pub health: f32,
    pub stamina: f32,
    pub dodge: f32,
    pub damage: f32,
    pub armor_multiplier: f32,
    pub loot: Vec<LootDrop>,
    regen_delay: f32,
    dodge_dir: Vec2,
    pub kills: u32,
    pub swing: f32,
    pub attack_dir: Vec2,
    pub invulnerable: f32,
    pub cooldown: f32,
    pub effects: Vec<HitEffect>,
}

pub fn is_safe(pos: Vec2) -> bool {
    village_distance(pos) <= SAFE_RADIUS
}

fn village_distance(pos: Vec2) -> f32 {
    pos.distance(BRIARGLEN).min(pos.distance(WILLOWFORD))
}

fn direction(facing: u8) -> Vec2 {
    match facing {
        1 => -Vec2::X,
        2 => Vec2::X,
        3 => -Vec2::Y,
        _ => Vec2::Y,
    }
}

fn facing(dir: Vec2) -> u8 {
    if dir.x.abs() > dir.y.abs() {
        if dir.x < 0.0 { 1 } else { 2 }
    } else if dir.y < 0.0 {
        3
    } else {
        0
    }
}

fn clear_line(world: &World, from: Vec2, to: Vec2) -> bool {
    let steps = (from.distance(to) / 3.0).ceil().max(1.0) as usize;
    (0..=steps).all(|i| world.can_walk(from.lerp(to, i as f32 / steps as f32), 1.0))
}

/// Keep the whole collision footprint outside wards and inside its home range.
fn move_monster(world: &World, monster: &mut Monster, delta: Vec2) {
    let steps = (delta.length() / 2.0).ceil().max(1.0) as usize;
    let step = delta / steps as f32;
    for _ in 0..steps {
        let next = world.slide_move(monster.pos, step);
        if village_distance(next) < SAFE_RADIUS + PLAYER_RADIUS
            || next.distance(monster.home) > HOME_LEASH
        {
            break;
        }
        monster.pos = next;
    }
}

impl Combat {
    pub fn new(world: &World) -> Self {
        let mut monsters = Vec::new();
        // Four bands leave the road and village approaches peaceful. A small
        // deterministic spiral finds clear ground among the generated trees.
        for (row, y) in [240.0, 500.0, 1480.0, 1810.0].into_iter().enumerate() {
            for (column, x) in [180.0, 560.0, 970.0, 1580.0, 1980.0, 2350.0]
                .into_iter()
                .enumerate()
            {
                let anchor = Vec2::new(x, y);
                let position = (0..160).find_map(|i| {
                    let angle = i as f32 * 2.399_963;
                    let radius = (i as f32).sqrt() * 6.0;
                    let pos = anchor + Vec2::new(angle.cos(), angle.sin()) * radius;
                    (village_distance(pos) >= 405.0 && world.can_walk(pos, 9.0)).then_some(pos)
                });
                if let Some(pos) = position {
                    let kind = if (column + row) % 3 == 0 {
                        Kind::Wolf
                    } else {
                        Kind::Slime
                    };
                    monsters.push(Monster::new(kind, pos));
                }
            }
        }
        monsters.push(Monster::new(Kind::Guardian, GUARDIAN_HOME));
        Self {
            monsters,
            hostility: [0.0; 2],
            residents: world.npcs.iter().map(|_| Resident::new()).collect(),
            health: MAX_HEALTH,
            stamina: MAX_STAMINA,
            dodge: 0.0,
            damage: 25.0,
            armor_multiplier: 1.0,
            loot: Vec::new(),
            regen_delay: 0.0,
            dodge_dir: Vec2::Y,
            kills: 0,
            swing: 0.0,
            attack_dir: Vec2::Y,
            invulnerable: 0.0,
            cooldown: 0.0,
            effects: Vec::new(),
        }
    }

    /// Running consumes stamina once per movement update, even against a wall.
    pub fn movement_speed(&mut self, running: bool, moving: bool, dt: f32) -> f32 {
        if self.health <= 0.0 || self.dodge > 0.0 {
            return 0.0;
        }
        if !dt.is_finite() || dt <= 0.0 {
            return 65.0;
        }
        let cost = 14.0 * dt.min(0.1);
        if running && moving && self.stamina >= cost {
            self.stamina = (self.stamina - cost).max(0.0);
            self.regen_delay = 0.6;
            112.0
        } else {
            65.0
        }
    }

    pub fn try_dodge(&mut self, direction: Vec2) -> bool {
        if self.health <= 0.0 || self.dodge > 0.0 || self.stamina < 30.0 || !direction.is_finite() {
            return false;
        }
        let Some(dir) = direction.try_normalize() else {
            return false;
        };
        self.stamina -= 30.0;
        self.regen_delay = 0.6;
        self.dodge = DODGE_DURATION;
        self.dodge_dir = dir;
        self.invulnerable = self.invulnerable.max(0.25);
        self.swing = 0.0;
        true
    }

    pub fn dodge_direction(&self) -> Vec2 {
        self.dodge_dir
    }

    pub fn take_loot(&mut self, pos: Vec2) -> (u32, u32) {
        let mut collected = (0_u32, 0_u32);
        self.loot.retain(|drop| {
            if drop.life > 0.0 && drop.pos.distance_squared(pos) <= 22.0_f32.powi(2) {
                collected.0 = collected.0.saturating_add(drop.coins);
                collected.1 = collected.1.saturating_add(drop.materials);
                false
            } else {
                true
            }
        });
        collected
    }

    pub fn guardian_defeated(&self) -> bool {
        self.monsters
            .iter()
            .any(|monster| monster.kind == Kind::Guardian && monster.health <= 0.0)
    }

    /// Restoring a save changes boss state without awarding drops or kill credit.
    pub fn restore_guardian(&mut self, dead: bool) {
        for monster in &mut self.monsters {
            if monster.kind == Kind::Guardian {
                *monster = Monster::new(Kind::Guardian, monster.home);
                if dead {
                    monster.health = 0.0;
                }
            }
        }
    }

    /// Resolve each target once when a swing starts; animation never deals damage.
    pub fn attack(&mut self, world: &World, player_pos: Vec2, player_facing: u8) -> bool {
        if self.cooldown > 0.0
            || self.health <= 0.0
            || self.stamina < 8.0
            || self.dodge > 0.0
            || !player_pos.is_finite()
        {
            return false;
        }
        self.stamina -= 8.0;
        self.regen_delay = 0.6;
        self.cooldown = 0.4;
        self.swing = SWING_DURATION;
        let strike_damage = if self.damage.is_finite() {
            self.damage.max(1.0)
        } else {
            25.0
        };
        self.attack_dir = direction(player_facing);
        for monster in &mut self.monsters {
            let offset = monster.pos - player_pos;
            let distance = offset.length();
            if monster.health <= 0.0
                || distance > 42.0
                || (distance > 0.01 && offset.dot(self.attack_dir) / distance < 0.5)
                || !clear_line(world, player_pos, monster.pos)
            {
                continue;
            }
            let damage = monster.health.min(strike_damage);
            monster.health = (monster.health - strike_damage).max(0.0);
            monster.hit_flash = 0.18;
            if monster.kind != Kind::Guardian {
                monster.windup = 0.0;
                monster.bite_cooldown = monster.bite_cooldown.max(0.45);
            }
            self.effects.push(HitEffect {
                pos: monster.pos,
                amount: damage as u32,
                life: 0.8,
                player: false,
            });
            let knockback = if monster.kind == Kind::Guardian {
                2.0
            } else {
                12.0
            };
            move_monster(world, monster, self.attack_dir * knockback);
            if monster.health <= 0.0 {
                monster.respawn = RESPAWN_TIME;
                monster.windup = 0.0;
                monster.walking = false;
                self.kills = self.kills.saturating_add(1);
                let (coins, materials) = match monster.kind {
                    Kind::Slime => (3, 1),
                    Kind::Wolf => (6, 2),
                    Kind::Guardian => (40, 5),
                };
                self.loot.push(LootDrop {
                    pos: monster.pos,
                    coins,
                    materials,
                    life: 180.0,
                });
            }
        }
        for (index, npc) in world.npcs.iter().enumerate() {
            let offset = npc.pos - player_pos;
            let distance = offset.length();
            if distance > 42.0
                || (distance > 0.01 && offset.dot(self.attack_dir) / distance < 0.5)
                || !clear_line(world, player_pos, npc.pos)
            {
                continue;
            }
            // Even striking a resident already on the ground renews the alarm.
            self.hostility[village(npc.home)] = 60.0;
            let resident = &mut self.residents[index];
            if resident.down > 0.0 {
                continue;
            }
            let damage = resident.health.min(strike_damage);
            resident.health = (resident.health - strike_damage).max(0.0);
            resident.hit_flash = 0.18;
            resident.windup = 0.0;
            resident.cooldown = resident.cooldown.max(0.45);
            self.effects.push(HitEffect {
                pos: npc.pos,
                amount: damage as u32,
                life: 0.8,
                player: false,
            });
            if resident.health <= 0.0 {
                resident.down = 30.0;
            }
        }
        true
    }

    pub fn resident_hostile(&self, index: usize, world: &World) -> bool {
        world
            .npcs
            .get(index)
            .is_some_and(|npc| self.hostility[village(npc.home)] > 0.0)
    }

    /// The parent handles the return to town when this returns true.
    pub fn update(
        &mut self,
        world: &mut World,
        player: &mut Player,
        dt: f32,
        elapsed: f32,
    ) -> bool {
        if self.health <= 0.0 {
            return true;
        }
        if !dt.is_finite() || !elapsed.is_finite() || !player.pos.is_finite() {
            return false;
        }
        // Lost frame time is discarded, matching world simulation; no unbounded
        // catch-up or instant bites when resuming after a suspended window.
        let dt = dt.clamp(0.0, 0.1);
        if dt == 0.0 {
            return false;
        }
        if self.dodge > 0.0 {
            let before = player.pos;
            player.pos = world.slide_move(player.pos, self.dodge_dir * 250.0 * self.dodge.min(dt));
            player.walking = player.pos.distance_squared(before) > 0.001;
            player.facing = facing(self.dodge_dir);
            self.dodge = (self.dodge - dt).max(0.0);
        }
        let recovering_time = (dt - self.regen_delay).max(0.0);
        self.regen_delay = (self.regen_delay - dt).max(0.0);
        self.stamina = (self.stamina + 24.0 * recovering_time).clamp(0.0, MAX_STAMINA);
        self.loot.iter_mut().for_each(|drop| drop.life -= dt);
        self.loot.retain(|drop| drop.life > 0.0);
        let armor = if self.armor_multiplier.is_finite() {
            self.armor_multiplier.clamp(0.0, 1.0)
        } else {
            1.0
        };
        self.swing = (self.swing - dt).max(0.0);
        self.cooldown = (self.cooldown - dt).max(0.0);
        self.invulnerable = (self.invulnerable - dt).max(0.0);
        self.effects.iter_mut().for_each(|effect| effect.life -= dt);
        self.effects.retain(|effect| effect.life > 0.0);
        for (index, hostility) in self.hostility.iter_mut().enumerate() {
            if player.pos.distance(village_center(index)) > 400.0 {
                *hostility = (*hostility - dt).max(0.0);
            }
        }
        if is_safe(player.pos) && self.hostility[village(player.pos)] <= 0.0 {
            self.health = (self.health + 12.0 * dt).min(MAX_HEALTH);
        }
        for (index, monster) in self.monsters.iter_mut().enumerate() {
            monster.walking = false;
            monster.hit_flash = (monster.hit_flash - dt).max(0.0);
            monster.bite_cooldown = (monster.bite_cooldown - dt).max(0.0);
            if monster.health <= 0.0 {
                monster.respawn = (monster.respawn - dt).max(0.0);
                if monster.kind != Kind::Guardian
                    && monster.respawn <= 0.0
                    && player.pos.distance(monster.home) > 120.0
                {
                    *monster = Monster::new(monster.kind, monster.home);
                }
                continue;
            }
            if monster.hit_flash > 0.0 && monster.kind != Kind::Guardian {
                continue;
            }
            let offset = player.pos - monster.pos;
            let distance = offset.length();
            if monster.windup > 0.0 {
                monster.windup = (monster.windup - dt).max(0.0);
                if monster.windup <= 0.0 {
                    monster.bite_cooldown = if monster.kind == Kind::Guardian {
                        1.5
                    } else {
                        1.2
                    };
                    if !is_safe(player.pos)
                        && distance <= monster.kind.reach()
                        && (distance < 0.01 || offset.dot(monster.bite_dir) / distance >= 0.2)
                        && self.invulnerable <= 0.0
                        && clear_line(world, monster.pos, player.pos)
                    {
                        let damage = self.health.min((monster.kind.damage() * armor).max(1.0));
                        self.health = (self.health - damage).max(0.0);
                        self.invulnerable = 0.8;
                        self.effects.push(HitEffect {
                            pos: player.pos,
                            amount: damage as u32,
                            life: 0.8,
                            player: true,
                        });
                        player.pos = world.slide_move(player.pos, monster.bite_dir * 14.0);
                        if self.health <= 0.0 {
                            return true;
                        }
                    }
                }
                continue;
            }
            let chasing = !is_safe(player.pos)
                && distance
                    <= if monster.kind == Kind::Guardian {
                        190.0
                    } else {
                        155.0
                    }
                && player.pos.distance(monster.home) <= HOME_LEASH
                && clear_line(world, monster.pos, player.pos);
            if chasing && distance <= monster.kind.reach() - 7.0 && monster.bite_cooldown <= 0.0 {
                monster.windup = monster.kind.windup_duration();
                monster.bite_dir = offset.try_normalize().unwrap_or(Vec2::Y);
                monster.facing = facing(monster.bite_dir);
                continue;
            }
            let target = if chasing {
                player.pos
            } else if monster.kind == Kind::Guardian || monster.pos.distance(monster.home) > 76.0 {
                monster.home
            } else {
                let phase = ((elapsed + index as f32 * 1.7) / 6.0).floor();
                let angle = phase * 2.399_963 + index as f32 * 1.618;
                monster.home + Vec2::new(angle.cos(), angle.sin()) * 52.0
            };
            if let Some(dir) = (target - monster.pos).try_normalize() {
                // Keep room for readable windups instead of overlapping the player.
                if target.distance(monster.pos) > 3.0 && (!chasing || distance > 19.0) {
                    let before = monster.pos;
                    let speed = monster.kind.speed() * if chasing { 1.0 } else { 0.4 };
                    move_monster(world, monster, dir * speed * dt);
                    monster.walking = before.distance_squared(monster.pos) > 0.001;
                    if monster.walking {
                        monster.facing = facing(dir);
                    }
                }
            }
        }
        self.update_residents(world, player, dt, armor)
    }

    fn update_residents(
        &mut self,
        world: &mut World,
        player: &mut Player,
        dt: f32,
        armor: f32,
    ) -> bool {
        for index in 0..world.npcs.len() {
            let resident = &mut self.residents[index];
            resident.hit_flash = (resident.hit_flash - dt).max(0.0);
            resident.cooldown = (resident.cooldown - dt).max(0.0);
            let npc = &world.npcs[index];
            let pos = npc.pos;
            let home = npc.home;
            let center = village_center(village(home));
            let hostile = self.hostility[village(home)] > 0.0;
            if resident.down > 0.0 {
                resident.down = (resident.down - dt).max(0.0);
                world.npcs[index].walking = false;
                if resident.down <= 0.0 {
                    *resident = Resident::new();
                }
                continue;
            }
            if !hostile {
                resident.windup = 0.0;
                resident.health = (resident.health + dt * 6.0).min(RESIDENT_HEALTH);
                continue;
            }
            world.npcs[index].walking = false;
            if resident.hit_flash > 0.0 {
                continue;
            }
            let offset = player.pos - pos;
            let distance = offset.length();
            if resident.windup > 0.0 {
                resident.windup = (resident.windup - dt).max(0.0);
                if resident.windup <= 0.0 {
                    resident.cooldown = 1.2;
                    if distance <= 30.0
                        && self.invulnerable <= 0.0
                        && (distance < 0.01 || offset.dot(resident.bite_dir) / distance >= 0.2)
                        && clear_line(world, pos, player.pos)
                    {
                        let damage = self.health.min((10.0 * armor).max(1.0));
                        self.health = (self.health - damage).max(0.0);
                        self.invulnerable = 0.8;
                        self.effects.push(HitEffect {
                            pos: player.pos,
                            amount: damage as u32,
                            life: 0.8,
                            player: true,
                        });
                        player.pos = world.slide_move(player.pos, resident.bite_dir * 14.0);
                        if self.health <= 0.0 {
                            return true;
                        }
                    }
                }
                continue;
            }
            let chasing = player.pos.distance(center) < 420.0;
            if chasing
                && distance <= 23.0
                && resident.cooldown <= 0.0
                && clear_line(world, pos, player.pos)
            {
                resident.windup = 0.4;
                resident.bite_dir = offset.try_normalize().unwrap_or(Vec2::Y);
                world.npcs[index].facing = facing(resident.bite_dir);
                continue;
            }
            let target = if chasing { player.pos } else { home };
            if let Some(dir) = (target - pos).try_normalize()
                && target.distance(pos) > 3.0
                && (!chasing || distance > 19.0)
            {
                let next = world.slide_move(pos, dir * 58.0 * dt);
                if next.distance(center) <= 420.0 {
                    world.npcs[index].pos = next;
                    world.npcs[index].walking = next.distance_squared(pos) > 0.001;
                    if world.npcs[index].walking {
                        world.npcs[index].facing = facing(dir);
                    }
                }
            }
        }
        false
    }

    pub fn recover(&mut self) {
        self.health = MAX_HEALTH;
        self.stamina = MAX_STAMINA;
        self.regen_delay = 0.0;
        self.dodge = 0.0;
        self.invulnerable = 2.0;
        self.cooldown = 0.0;
        self.swing = 0.0;
        self.effects.clear();
        self.hostility = [0.0; 2];
        self.residents
            .iter_mut()
            .for_each(|resident| *resident = Resident::new());
        for monster in &mut self.monsters {
            monster.windup = 0.0;
            monster.bite_cooldown = monster.bite_cooldown.max(1.2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{HEIGHT, Prop, PropKind, TILE, Tile, WIDTH};

    fn meadow() -> World {
        World {
            tiles: vec![Tile::Grass; WIDTH * HEIGHT],
            props: vec![],
            npcs: vec![],
        }
    }

    fn encounter(world: &World, positions: &[Vec2]) -> Combat {
        let mut combat = Combat::new(world);
        combat.monsters = positions
            .iter()
            .map(|&pos| Monster::new(Kind::Wolf, pos))
            .collect();
        combat
    }

    fn tick(combat: &mut Combat, world: &mut World, player: &mut Player, seconds: f32) {
        for frame in 0..(seconds / 0.05).ceil() as usize {
            combat.update(world, player, 0.05, frame as f32 * 0.05);
        }
    }

    #[test]
    fn homes_are_deterministic_walkable_and_far_from_both_villages() {
        let world = World::new();
        let first = Combat::new(&world);
        let second = Combat::new(&world);
        assert_eq!(first.monsters.len(), 25);
        for (a, b) in first.monsters.iter().zip(&second.monsters) {
            assert_eq!(a.home, b.home);
            assert_eq!(a.kind, b.kind);
            assert!(village_distance(a.home) >= 400.0);
            assert!(world.can_walk(a.home, 9.0));
        }
        assert!(is_safe(BRIARGLEN));
        assert!(is_safe(WILLOWFORD + Vec2::X * SAFE_RADIUS));
        assert!(!is_safe(Vec2::new(200.0, 200.0)));
    }

    #[test]
    fn sword_respects_arc_range_cooldown_and_hits_each_target_once() {
        let mut world = meadow();
        let center = Vec2::new(300.0, 300.0);
        let mut combat = encounter(
            &world,
            &[
                center + Vec2::X * 25.0,
                center - Vec2::X * 25.0,
                center + Vec2::Y * 25.0,
                center + Vec2::X * 44.0,
            ],
        );
        assert!(combat.attack(&world, center, 2));
        assert_eq!(combat.monsters[0].health, 40.0);
        assert!(combat.monsters[1..].iter().all(|m| m.health == 65.0));
        assert!(!combat.attack(&world, center, 2));
        assert_eq!(combat.monsters[0].health, 40.0);
        let mut player = Player {
            pos: center,
            facing: 2,
            walking: false,
        };
        tick(&mut combat, &mut world, &mut player, 0.25);
        assert_eq!(combat.monsters[0].health, 40.0);
    }

    #[test]
    fn weapons_and_knockback_cannot_cross_obstacles() {
        let mut world = meadow();
        let from = Vec2::new(285.0, 300.0);
        let to = Vec2::new(320.0, 300.0);
        world.props.push(Prop {
            kind: PropKind::Tree,
            pos: Vec2::new(302.0, 303.0),
            variant: 0,
        });
        let mut combat = encounter(&world, &[to]);
        assert!(combat.attack(&world, from, 2));
        assert_eq!(combat.monsters[0].health, 65.0);
        world.props.clear();
        world.tiles[18 * WIDTH + 19] = Tile::Water;
        combat.cooldown = 0.0;
        assert!(combat.attack(&world, from, 2));
        assert_eq!(combat.monsters[0].health, 65.0);
        let mut monster = Monster::new(Kind::Wolf, from);
        move_monster(&world, &mut monster, Vec2::X * 100.0);
        assert!(monster.pos.x < 19.0 * TILE);
        assert!(world.can_walk(monster.pos, PLAYER_RADIUS));
    }

    #[test]
    fn bites_telegraph_allow_dodges_and_share_player_invulnerability() {
        let mut world = meadow();
        let origin = Vec2::new(300.0, 300.0);
        let mut player = Player {
            pos: origin + Vec2::X * 20.0,
            facing: 1,
            walking: false,
        };
        let mut combat = encounter(&world, &[origin, origin + Vec2::Y]);
        tick(&mut combat, &mut world, &mut player, 0.35);
        assert_eq!(combat.health, MAX_HEALTH);
        assert!(combat.monsters[0].windup > 0.0);
        tick(&mut combat, &mut world, &mut player, 0.15);
        assert_eq!(combat.health, MAX_HEALTH - 18.0);
        assert!(combat.invulnerable > 0.0);
        combat.recover();
        assert_eq!(combat.health, MAX_HEALTH);
        assert!(combat.effects.is_empty());
        combat.invulnerable = 0.0;
        combat.monsters = vec![Monster::new(Kind::Wolf, origin)];
        player.pos = origin + Vec2::X * 20.0;
        tick(&mut combat, &mut world, &mut player, 0.1);
        player.pos += Vec2::X * 45.0;
        tick(&mut combat, &mut world, &mut player, 0.4);
        assert_eq!(combat.health, MAX_HEALTH);
    }

    #[test]
    fn wards_home_leashes_and_river_block_pursuit() {
        let mut world = meadow();
        let home = BRIARGLEN - Vec2::X * 405.0;
        let mut combat = encounter(&world, &[home]);
        let mut player = Player {
            pos: BRIARGLEN - Vec2::X * 321.0,
            facing: 1,
            walking: false,
        };
        tick(&mut combat, &mut world, &mut player, 15.0);
        assert!(!is_safe(combat.monsters[0].pos));
        assert!(combat.monsters[0].pos.distance(home) <= HOME_LEASH);
        player.pos = BRIARGLEN;
        combat.health = 50.0;
        tick(&mut combat, &mut world, &mut player, 1.0);
        assert!((combat.health - 62.0).abs() < 0.01);
        player.pos = home - Vec2::X * 240.0;
        tick(&mut combat, &mut world, &mut player, 15.0);
        assert!(combat.monsters[0].pos.distance(home) <= HOME_LEASH);
        for row in 0..HEIGHT {
            world.tiles[row * WIDTH + 22] = Tile::Water;
        }
        combat = encounter(&world, &[Vec2::new(330.0, 300.0)]);
        player.pos = Vec2::new(385.0, 300.0);
        tick(&mut combat, &mut world, &mut player, 20.0);
        assert!(combat.monsters[0].pos.x < 22.0 * TILE);
        assert_eq!(combat.health, MAX_HEALTH);
    }

    #[test]
    fn kills_count_once_and_respawn_waits_until_player_leaves() {
        let mut world = meadow();
        let home = Vec2::new(300.0, 300.0);
        let mut combat = encounter(&world, &[home]);
        combat.monsters[0].health = 20.0;
        assert!(combat.attack(&world, home - Vec2::X * 20.0, 2));
        assert_eq!(combat.kills, 1);
        assert_eq!(combat.monsters[0].respawn, RESPAWN_TIME);
        let mut player = Player {
            pos: home,
            facing: 2,
            walking: false,
        };
        tick(&mut combat, &mut world, &mut player, 91.0);
        assert_eq!(combat.monsters[0].health, 0.0);
        assert_eq!(combat.kills, 1);
        player.pos = BRIARGLEN;
        tick(&mut combat, &mut world, &mut player, 0.1);
        assert_eq!(combat.monsters[0].health, 65.0);
        assert!(combat.monsters[0].pos.distance(home) < 3.0);
        combat.health = 0.0;
        assert!(combat.update(&mut world, &mut player, 0.05, 1.0));
        combat.recover();
        assert_eq!(combat.health, MAX_HEALTH);
        assert_eq!(combat.kills, 1);
    }
    #[test]
    fn provoking_one_resident_alerts_their_whole_village_only() {
        let mut world = World::new();
        let mut combat = Combat::new(&world);
        // Use clear village ground so this tests allegiance, not authored props.
        let npc_index = 0;
        let home = world.npcs[npc_index].home;
        let center = village_center(village(home));
        world.npcs[npc_index].pos = center;
        let mut player = Player {
            pos: center - Vec2::X * 20.0,
            facing: 2,
            walking: false,
        };
        assert!(!combat.resident_hostile(npc_index, &world));
        combat.health = 50.0;
        tick(&mut combat, &mut world, &mut player, 0.1);
        assert!(combat.health > 50.0);
        assert!(combat.attack(&world, player.pos, 2));
        assert_eq!(combat.residents[npc_index].health, RESIDENT_HEALTH - 25.0);
        for (index, npc) in world.npcs.iter().enumerate() {
            assert_eq!(
                combat.resident_hostile(index, &world),
                village(npc.home) == village(home)
            );
        }
        let health = combat.health;
        tick(&mut combat, &mut world, &mut player, 0.15);
        assert_eq!(
            combat.health, health,
            "hostile wards must not heal the attacker"
        );
        assert_eq!(combat.hostility[village(home)], 60.0);
        tick(&mut combat, &mut world, &mut player, 1.0);
        assert!(combat.health < health, "the provoked resident retaliates");
        player.pos = Vec2::new(100.0, 100.0);
        tick(&mut combat, &mut world, &mut player, 1.0);
        assert!(combat.hostility[village(home)] < 60.0);
        combat.recover();
        assert_eq!(combat.hostility, [0.0, 0.0]);
        assert!(
            combat
                .residents
                .iter()
                .all(|resident| resident.health == RESIDENT_HEALTH)
        );
    }

    #[test]
    fn defeated_residents_recover_without_counting_as_monster_kills() {
        let mut world = World::new();
        let center = BRIARGLEN;
        world.npcs[0].pos = center;
        let mut combat = Combat::new(&world);
        combat.residents[0].health = 10.0;
        assert!(combat.attack(&world, center - Vec2::X * 20.0, 2));
        assert_eq!(combat.residents[0].down, 30.0);
        assert_eq!(combat.kills, 0);
        let mut player = Player::new();
        player.pos = Vec2::new(100.0, 100.0);
        tick(&mut combat, &mut world, &mut player, 31.0);
        assert_eq!(combat.residents[0].down, 0.0);
        assert_eq!(combat.residents[0].health, RESIDENT_HEALTH);
    }
    #[test]
    fn residents_cannot_be_hit_or_retaliate_through_obstacles() {
        let mut world = World::new();
        world.props.clear();
        world.npcs.truncate(1);
        world.npcs[0].pos = BRIARGLEN;
        let barrier = Prop {
            kind: PropKind::Tree,
            pos: BRIARGLEN + Vec2::new(-10.0, 3.0),
            variant: 0,
        };
        world.props.push(barrier.clone());
        let mut combat = Combat::new(&world);
        let mut player = Player {
            pos: BRIARGLEN - Vec2::X * 20.0,
            facing: 2,
            walking: false,
        };
        assert!(combat.attack(&world, player.pos, 2));
        assert_eq!(combat.residents[0].health, RESIDENT_HEALTH);
        assert_eq!(combat.hostility, [0.0, 0.0]);
        combat.hostility[0] = 60.0;
        combat.health = 50.0;
        tick(&mut combat, &mut world, &mut player, 0.6);
        assert_eq!(combat.residents[0].windup, 0.0);
        assert_eq!(combat.health, 50.0);
        world.props.clear();
        tick(&mut combat, &mut world, &mut player, 0.05);
        assert!(combat.residents[0].windup > 0.0);
        world.props.push(barrier);
        tick(&mut combat, &mut world, &mut player, 0.45);
        assert_eq!(combat.health, 50.0);
    }
    #[test]
    fn stamina_costs_are_applied_once_and_regeneration_waits() {
        let mut world = meadow();
        let mut combat = encounter(&world, &[]);
        let mut player = Player::new();
        assert_eq!(combat.movement_speed(true, false, 0.1), 65.0);
        assert_eq!(combat.stamina, MAX_STAMINA);
        combat.stamina = 50.0;
        assert_eq!(combat.movement_speed(true, true, 0.1), 112.0);
        assert!((combat.stamina - 48.6).abs() < 0.001);
        tick(&mut combat, &mut world, &mut player, 0.5);
        assert!((combat.stamina - 48.6).abs() < 0.001);
        tick(&mut combat, &mut world, &mut player, 0.2);
        assert!((combat.stamina - 51.0).abs() < 0.001);
        assert!(combat.attack(&world, player.pos, 0));
        assert!((combat.stamina - 43.0).abs() < 0.001);
        assert!(!combat.attack(&world, player.pos, 0));
        assert!((combat.stamina - 43.0).abs() < 0.001);
        combat.stamina = 7.0;
        combat.cooldown = 0.0;
        assert!(!combat.attack(&world, player.pos, 0));
        assert!(!combat.try_dodge(Vec2::X));
        combat.stamina = 0.0;
        assert_eq!(combat.movement_speed(true, true, 0.1), 65.0);
        combat.recover();
        assert_eq!(combat.stamina, MAX_STAMINA);
    }

    #[test]
    fn dodge_has_a_bounded_distance_and_cannot_tunnel_through_water() {
        let mut world = meadow();
        let start = Vec2::new(285.0, 300.0);
        let mut combat = encounter(&world, &[]);
        let mut player = Player {
            pos: start,
            facing: 0,
            walking: false,
        };
        assert!(!combat.try_dodge(Vec2::ZERO));
        assert!(!combat.try_dodge(Vec2::splat(f32::NAN)));
        assert_eq!(combat.stamina, MAX_STAMINA);
        assert!(combat.try_dodge(Vec2::X * 3.0));
        assert_eq!(combat.stamina, 70.0);
        assert_eq!(combat.invulnerable, 0.25);
        assert!(!combat.try_dodge(Vec2::X));
        assert!(!combat.attack(&world, player.pos, 2));
        assert_eq!(combat.movement_speed(true, true, 0.1), 0.0);
        tick(&mut combat, &mut world, &mut player, 0.3);
        assert!((player.pos.distance(start) - 55.0).abs() < 0.001);
        assert_eq!(combat.dodge, 0.0);
        world.tiles[18 * WIDTH + 19] = Tile::Water;
        player.pos = start;
        assert!(combat.try_dodge(Vec2::X));
        tick(&mut combat, &mut world, &mut player, 0.3);
        assert!(player.pos.x < 19.0 * TILE);
        assert!(world.can_walk(player.pos, PLAYER_RADIUS));
    }

    #[test]
    fn monster_loot_can_be_collected_once_and_residents_drop_nothing() {
        let world = meadow();
        let origin = Vec2::new(300.0, 300.0);
        let mut combat = encounter(&world, &[origin]);
        combat.damage = 70.0;
        assert!(combat.attack(&world, origin - Vec2::X * 20.0, 2));
        assert_eq!(combat.loot.len(), 1);
        assert_eq!(combat.take_loot(origin - Vec2::X * 30.0), (0, 0));
        assert_eq!(combat.take_loot(origin), (6, 2));
        assert_eq!(combat.take_loot(origin), (0, 0));
        combat.cooldown = 0.0;
        assert!(combat.attack(&world, origin, 2));
        assert_eq!(combat.kills, 1);
        assert!(combat.loot.is_empty());
        let mut village = World::new();
        village.npcs.truncate(1);
        village.npcs[0].pos = BRIARGLEN;
        let mut combat = Combat::new(&village);
        combat.damage = 70.0;
        assert!(combat.attack(&village, BRIARGLEN - Vec2::X * 20.0, 2));
        assert!(combat.residents[0].down > 0.0);
        assert!(combat.loot.is_empty());
    }

    #[test]
    fn armor_reduces_monster_and_resident_damage_with_a_one_point_floor() {
        let mut world = meadow();
        let origin = Vec2::new(300.0, 300.0);
        let mut combat = encounter(&world, &[origin]);
        combat.armor_multiplier = 0.5;
        let mut player = Player {
            pos: origin + Vec2::X * 20.0,
            facing: 1,
            walking: false,
        };
        tick(&mut combat, &mut world, &mut player, 0.6);
        assert_eq!(combat.health, MAX_HEALTH - 9.0);
        let mut world = World::new();
        world.npcs.truncate(1);
        world.npcs[0].pos = BRIARGLEN;
        let mut combat = Combat::new(&world);
        combat.hostility[0] = 60.0;
        combat.armor_multiplier = 0.5;
        player.pos = BRIARGLEN + Vec2::X * 20.0;
        tick(&mut combat, &mut world, &mut player, 0.6);
        assert_eq!(combat.health, MAX_HEALTH - 5.0);
        combat.armor_multiplier = 0.0;
        combat.invulnerable = 0.0;
        combat.residents[0].cooldown = 0.0;
        player.pos = BRIARGLEN + Vec2::X * 20.0;
        tick(&mut combat, &mut world, &mut player, 0.6);
        assert_eq!(combat.health, MAX_HEALTH - 6.0);
    }

    #[test]
    fn guardian_state_restores_without_loot_and_never_respawns() {
        let mut world = meadow();
        let mut combat = Combat::new(&world);
        combat
            .monsters
            .retain(|monster| monster.kind == Kind::Guardian);
        assert_eq!(combat.monsters[0].home, GUARDIAN_HOME);
        assert!(!combat.guardian_defeated());
        combat.restore_guardian(true);
        assert!(combat.guardian_defeated());
        let mut player = Player::new();
        tick(&mut combat, &mut world, &mut player, 0.2);
        assert!(combat.guardian_defeated());
        assert_eq!(combat.kills, 0);
        assert!(combat.loot.is_empty());
        combat.restore_guardian(false);
        assert_eq!(combat.monsters[0].health, 180.0);
        combat.damage = 200.0;
        assert!(combat.attack(&world, GUARDIAN_HOME - Vec2::X * 20.0, 2));
        assert_eq!(combat.kills, 1);
        assert_eq!(combat.take_loot(GUARDIAN_HOME), (40, 5));
        combat.monsters[0].respawn = 0.05;
        tick(&mut combat, &mut world, &mut player, 0.2);
        assert!(combat.guardian_defeated());
        assert_eq!(combat.monsters[0].respawn, 0.0);
        combat.recover();
        assert!(
            combat.guardian_defeated(),
            "rescue must preserve a defeated boss"
        );
    }

    #[test]
    fn guardian_long_windup_survives_sword_hits_and_reaches_farther() {
        let mut world = meadow();
        let mut combat = Combat::new(&world);
        combat
            .monsters
            .retain(|monster| monster.kind == Kind::Guardian);
        let mut player = Player {
            pos: GUARDIAN_HOME + Vec2::X * 40.0,
            facing: 1,
            walking: false,
        };
        tick(&mut combat, &mut world, &mut player, 0.05);
        assert_eq!(combat.monsters[0].windup, 0.7);
        assert!(combat.attack(&world, player.pos, 1));
        assert_eq!(combat.monsters[0].windup, 0.7);
        tick(&mut combat, &mut world, &mut player, 0.5);
        assert_eq!(combat.health, MAX_HEALTH);
        tick(&mut combat, &mut world, &mut player, 0.25);
        assert_eq!(combat.health, MAX_HEALTH - 22.0);
    }
    #[test]
    fn guardian_can_be_dodged_inside_the_authored_dungeon() {
        let mut world = World::new();
        let mut combat = Combat::new(&world);
        combat
            .monsters
            .retain(|monster| monster.kind == Kind::Guardian);
        let mut player = Player {
            pos: GUARDIAN_HOME + Vec2::X * 40.0,
            facing: 1,
            walking: false,
        };
        tick(&mut combat, &mut world, &mut player, 0.55);
        assert!(combat.monsters[0].windup > 0.0);
        assert!(combat.try_dodge(Vec2::Y));
        tick(&mut combat, &mut world, &mut player, 0.3);
        assert_eq!(combat.health, MAX_HEALTH);
        assert!(world.can_walk(player.pos, PLAYER_RADIUS));
        assert!(player.pos.y > GUARDIAN_HOME.y + 50.0);
        player.pos = Vec2::new(2088.0, 344.0);
        assert!(combat.try_dodge(Vec2::X));
        tick(&mut combat, &mut world, &mut player, 0.3);
        assert!(
            player.pos.x < 2096.0,
            "the sealed treasury gate blocks a dash"
        );
        assert!(world.can_walk(player.pos, PLAYER_RADIUS));
    }
}
