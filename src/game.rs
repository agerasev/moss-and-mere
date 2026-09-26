use crate::{
    combat::Combat,
    save::Save,
    world::{BRIARGLEN, Dialogue, Player, Quest, START, WILLOWFORD, World},
};
use wgame::glam::Vec2;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    None,
    Map,
    Journal,
    Help,
    Pause,
}

pub struct Game {
    pub world: World,
    pub combat: Combat,
    pub player: Player,
    pub quest: Quest,
    pub panel: Panel,
    pub dialogue: Option<Dialogue>,
    pub elapsed: f32,
    pub visited: u8,
    pub camera: Vec2,
    pub toast: String,
    pub toast_time: f32,
    pub save_time: f32,
    pub zoom: f32,
    pub save_enabled: bool,
}

impl Game {
    pub fn new(fresh: bool) -> Self {
        let world = World::new();
        let mut combat = Combat::new(&world);
        let saved = if fresh { None } else { Save::load() };
        let mut player = Player::new();
        let mut quest = Quest::NotStarted;
        let mut elapsed = 0.;
        let mut visited = 1;
        if let Some(save) = saved {
            player.pos = if world.can_walk(save.pos, 5.) {
                save.pos
            } else {
                START
            };
            quest = save.quest;
            elapsed = save.elapsed;
            visited = save.visited;
            combat.health = save.health;
            combat.kills = save.kills;
            for i in 0..2 {
                if save.hostile & (1 << i) != 0 {
                    combat.hostility[i] = 60.;
                }
            }
        }
        Self {
            camera: player.pos - Vec2::new(0., 24.),
            world,
            combat,
            player,
            quest,
            panel: Panel::None,
            dialogue: None,
            elapsed,
            visited,
            toast: "Welcome to the Green March".into(),
            toast_time: 6.,
            save_time: 0.,
            zoom: 2.0,
            save_enabled: true,
        }
    }

    pub fn nearest_npc(&self) -> Option<usize> {
        self.world
            .npcs
            .iter()
            .enumerate()
            .filter_map(|(i, n)| {
                let d = n.pos.distance(self.player.pos);
                (d < 34.
                    && self.combat.residents[i].down <= 0.
                    && !self.combat.resident_hostile(i, &self.world))
                .then_some((i, d))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }

    pub fn interact(&mut self) {
        if self.dialogue.is_some() {
            self.dialogue = None;
            return;
        }
        if self.panel != Panel::None {
            return;
        }
        if let Some(index) = self.nearest_npc() {
            let before = self.quest;
            self.dialogue = Some(self.world.interact(index, &mut self.quest));
            if before != self.quest {
                self.toast = match self.quest {
                    Quest::Carrying => "Journal updated · A little kindness",
                    Quest::Delivered => "Delivery complete · Friend of the March",
                    _ => "Journal updated",
                }
                .into();
                self.toast_time = 5.;
                self.persist();
            }
        }
    }

    pub fn attack(&mut self, aim: Option<Vec2>) {
        if self.panel != Panel::None || self.dialogue.is_some() {
            return;
        }
        if self.combat.cooldown > 0. {
            return;
        }
        if let Some(direction) = aim.filter(|d| d.length_squared() > 1.) {
            self.player.facing = crate::world::facing(direction);
        }
        let kills = self.combat.kills;
        let hostility = self.combat.hostility;
        self.combat
            .attack(&self.world, self.player.pos, self.player.facing);
        if let Some(village) =
            (0..2).find(|i| hostility[*i] <= 0. && self.combat.hostility[*i] > 0.)
        {
            self.toast = format!(
                "{} is hostile! The whole village is defending its people.",
                if village == 0 {
                    "Briarglen"
                } else {
                    "Willowford"
                }
            );
            self.toast_time = 7.;
            self.persist();
        } else if self.combat.kills > kills {
            self.toast = "Monster defeated · The wilds are a little safer".into();
            self.toast_time = 3.;
            self.persist();
        }
    }

    pub fn toggle(&mut self, panel: Panel) {
        self.dialogue = None;
        self.panel = if self.panel == panel {
            Panel::None
        } else {
            panel
        };
    }

    pub fn update(&mut self, dt: f32, direction: Vec2, running: bool) {
        self.toast_time = (self.toast_time - dt).max(0.);
        self.player.walking = false;
        if self.panel != Panel::None || self.dialogue.is_some() {
            return;
        }
        self.elapsed += dt;
        self.save_time += dt;
        if direction.length_squared() > 0. {
            let dir = direction.normalize();
            self.player.facing = if dir.x.abs() > dir.y.abs() {
                if dir.x < 0. { 1 } else { 2 }
            } else if dir.y < 0. {
                3
            } else {
                0
            };
            let next = self
                .world
                .slide_move(self.player.pos, dir * dt * if running { 112. } else { 65. });
            self.player.walking = next.distance_squared(self.player.pos) > 0.01;
            self.player.pos = next;
        }
        // Hostile or knocked-out residents are moved exclusively by combat AI.
        let controlled: Vec<_> = self
            .world
            .npcs
            .iter()
            .enumerate()
            .filter_map(|(i, n)| {
                (self.combat.resident_hostile(i, &self.world) || self.combat.residents[i].down > 0.)
                    .then_some((i, n.pos, n.facing, n.walking))
            })
            .collect();
        self.world.update(dt, self.elapsed);
        for (i, pos, facing, walking) in controlled {
            let npc = &mut self.world.npcs[i];
            npc.pos = pos;
            npc.facing = facing;
            npc.walking = walking;
        }
        if self
            .combat
            .update(&mut self.world, &mut self.player, dt, self.elapsed)
        {
            let willow = self.visited & 2 != 0
                && self.player.pos.distance(WILLOWFORD) < self.player.pos.distance(BRIARGLEN);
            self.player.pos = if willow {
                WILLOWFORD + Vec2::new(0., 40.)
            } else {
                START
            };
            self.player.walking = false;
            self.combat.recover();
            self.camera = self.player.pos - Vec2::new(0., 24.);
            self.toast = format!(
                "Rescued to {} · Your belongings are safe",
                if willow { "Willowford" } else { "Briarglen" }
            );
            self.toast_time = 7.;
            self.persist();
        }
        self.camera = self
            .camera
            .lerp(self.player.pos - Vec2::new(0., 24.), 1. - (-dt * 9.).exp());
        for (bit, pos, name) in [(1, BRIARGLEN, "Briarglen"), (2, WILLOWFORD, "Willowford")] {
            if self.visited & bit == 0 && self.player.pos.distance(pos) < 180. {
                self.visited |= bit;
                self.toast = format!("Discovered {name}");
                self.toast_time = 5.;
                self.persist();
            }
        }
        if self.save_time > 15. {
            self.persist();
        }
    }

    pub fn persist(&mut self) -> bool {
        if !self.save_enabled {
            return true;
        }
        self.save_time = 0.;
        if let Err(error) = (Save {
            pos: self.player.pos,
            quest: self.quest,
            visited: self.visited,
            elapsed: self.elapsed,
            health: self.combat.health,
            kills: self.combat.kills,
            hostile: u8::from(self.combat.hostility[0] > 0.)
                | (u8::from(self.combat.hostility[1] > 0.) << 1),
        })
        .store()
        {
            self.toast = format!("Could not save: {error}");
            self.toast_time = 30.;
            false
        } else {
            true
        }
    }

    pub fn objective(&self) -> &'static str {
        self.quest.objective()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game() -> Game {
        let mut game = Game::new(true);
        game.save_enabled = false;
        game
    }

    #[test]
    fn diagonal_movement_has_the_same_speed_and_panels_pause_the_world() {
        let mut straight = game();
        let mut diagonal = game();
        straight.update(0.05, Vec2::X, false);
        diagonal.update(0.05, Vec2::ONE, false);
        let a = straight.player.pos.distance(START);
        let b = diagonal.player.pos.distance(START);
        assert!((a - b).abs() < 0.001);
        diagonal.toggle(Panel::Map);
        let position = diagonal.player.pos;
        let elapsed = diagonal.elapsed;
        diagonal.update(1.0, Vec2::X, true);
        assert_eq!(diagonal.player.pos, position);
        assert_eq!(diagonal.elapsed, elapsed);
    }

    #[test]
    fn nearby_conversations_deliver_the_parcel_and_block_movement() {
        let mut game = game();
        game.interact();
        assert_eq!(game.quest, Quest::Carrying);
        assert_eq!(game.dialogue.as_ref().unwrap().speaker, "Mira");
        game.update(0.05, Vec2::X, true);
        assert_eq!(game.player.pos, START);
        game.interact();
        assert!(game.dialogue.is_none());
        let rowan = game
            .world
            .npcs
            .iter()
            .find(|npc| npc.name == "Rowan")
            .unwrap()
            .pos;
        game.player.pos = rowan + Vec2::new(10., 0.);
        game.interact();
        assert_eq!(game.quest, Quest::Delivered);
        assert_eq!(game.dialogue.as_ref().unwrap().speaker, "Rowan");
    }
    #[test]
    fn village_retaliation_blocks_dialogue_and_recovery_preserves_progress() {
        let mut game = game();
        game.quest = Quest::Carrying;
        let mira = game.world.npcs[0].pos;
        game.attack(Some(mira - game.player.pos));
        assert_eq!(game.combat.hostility, [60., 0.]);
        assert!(game.nearest_npc().is_none());
        game.interact();
        assert!(game.dialogue.is_none());
        let before: Vec<_> = game.world.npcs[..4]
            .iter()
            .map(|n| n.pos.distance(game.player.pos))
            .collect();
        for _ in 0..20 {
            game.update(0.05, Vec2::ZERO, false);
        }
        for (i, distance) in before.iter().enumerate().skip(1) {
            assert!(
                game.world.npcs[i].pos.distance(game.player.pos) < *distance,
                "resident {i} must join pursuit"
            );
        }
        game.toggle(Panel::Map);
        let health = game.combat.health;
        let positions: Vec<_> = game.world.npcs.iter().map(|n| n.pos).collect();
        game.attack(None);
        game.update(1., Vec2::ZERO, false);
        assert_eq!(game.combat.health, health);
        assert_eq!(
            positions,
            game.world.npcs.iter().map(|n| n.pos).collect::<Vec<_>>()
        );
        game.toggle(Panel::Map);
        game.combat.health = 0.;
        game.update(0.05, Vec2::ZERO, false);
        assert_eq!(game.player.pos, START);
        assert_eq!(game.combat.health, crate::combat::MAX_HEALTH);
        assert_eq!(game.combat.hostility, [0., 0.]);
        assert_eq!(game.quest, Quest::Carrying);
    }
}
