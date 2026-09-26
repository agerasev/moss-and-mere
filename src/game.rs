use crate::{
    audio::{Ambience, Cue},
    combat::Combat,
    progress::{ForgeQuest, Progress},
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
    Forge,
    Village,
}

pub struct Game {
    pub progress: Progress,
    pub cues: Vec<Cue>,
    pub muted: bool,
    running: bool,
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
        let mut world = World::new();
        let mut combat = Combat::new(&world);
        let saved = if fresh { None } else { Save::load() };
        let mut player = Player::new();
        let mut quest = Quest::NotStarted;
        let mut elapsed = 0.;
        let mut visited = 1;
        let mut progress = Progress::default();
        if let Some(save) = saved {
            progress = save.progress;
            if progress.secret_open {
                world.open_secret();
            }
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
        combat.restore_guardian(progress.guardian_defeated);
        combat.damage = progress.blade_damage();
        combat.armor_multiplier = progress.armor_multiplier();
        Self {
            progress,
            cues: Vec::new(),
            muted: false,
            running: false,
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
        if let Some(dialogue) = self.dialogue.take() {
            if dialogue.speaker == "Alden" {
                self.panel = Panel::Forge;
            }
            return;
        }
        if self.panel != Panel::None {
            return;
        }
        use crate::world::{RELIC_POS, SECRET_CHEST, SECRET_SWITCH};
        let pos = self.player.pos;
        if !self.progress.secret_open && pos.distance(SECRET_SWITCH) < 30. {
            self.world.open_secret();
            self.progress.secret_open = true;
            self.message(
                "The engraved stone shifts. A hidden chamber opens.",
                Cue::Quest,
            );
            self.persist();
        } else if self.progress.secret_open
            && !self.progress.chest_taken
            && pos.distance(SECRET_CHEST) < 28.
        {
            self.progress.chest_taken = true;
            self.progress.award_loot(30, 5);
            self.message("Hidden cache · 30 crowns and 5 iron shards", Cue::Loot);
            self.persist();
        } else if !self.progress.relic_taken && pos.distance(RELIC_POS) < 28. {
            if !self.progress.guardian_defeated {
                self.message(
                    "The Emberheart is bound to its guardian. Defeat it first.",
                    Cue::Talk,
                );
            } else {
                self.progress.relic_taken = true;
                self.progress.forge = ForgeQuest::Recovered;
                self.message(
                    "Emberheart recovered · Return to Alden in Briarglen",
                    Cue::Quest,
                );
                self.persist();
            }
        } else if let Some(index) = self.nearest_npc() {
            let before = self.quest;
            let forge = self.progress.forge;
            self.dialogue = Some(if index == crate::world::BLACKSMITH {
                self.progress.talk_blacksmith()
            } else {
                self.world.interact(index, &mut self.quest)
            });
            self.cues.push(Cue::Talk);
            if before != self.quest {
                if self.quest == Quest::Delivered {
                    self.progress.award_loot(30, 0);
                    self.progress.add_rep(0, 15);
                    self.progress.add_rep(1, 20);
                }
                self.message(
                    if self.quest == Quest::Delivered {
                        "Delivery complete · 30 crowns · Village trust increased"
                    } else {
                        "Journal updated · A little kindness"
                    },
                    Cue::Quest,
                );
                self.persist();
            }
            if forge != self.progress.forge {
                self.sync_equipment();
                self.cues.push(Cue::Quest);
                self.persist();
            }
        } else if self.near_village().is_some() {
            self.panel = Panel::Village;
        }
    }

    pub fn message(&mut self, text: impl Into<String>, cue: Cue) {
        self.toast = text.into();
        self.toast_time = 6.;
        self.cues.push(cue);
    }

    pub fn sync_equipment(&mut self) {
        self.combat.damage = self.progress.blade_damage();
        self.combat.armor_multiplier = self.progress.armor_multiplier();
    }

    pub fn near_village(&self) -> Option<usize> {
        [BRIARGLEN, WILLOWFORD]
            .iter()
            .position(|p| p.distance(self.player.pos) < 260.)
    }

    pub fn village_panel(&mut self) {
        if self.near_village().is_some() {
            self.toggle(Panel::Village);
        } else {
            self.message("Visit a village to speak with its council.", Cue::Talk);
        }
    }

    pub fn amends_cost(&self, village: usize) -> Option<u32> {
        self.progress
            .amends_cost(village)
            .or_else(|| (self.combat.hostility[village] > 0.).then_some(12))
    }

    pub fn menu_action(&mut self, choice: u8) {
        let result = match self.panel {
            Panel::Forge
                if self
                    .player
                    .pos
                    .distance(self.world.npcs[crate::world::BLACKSMITH].pos)
                    < 50.
                    && self.combat.hostility[0] <= 0. =>
            {
                match choice {
                    1 => self.progress.upgrade_blade(),
                    2 => self.progress.upgrade_armor(),
                    _ => return,
                }
            }
            Panel::Village if choice == 1 => {
                let Some(village) = self.near_village() else {
                    return;
                };
                let result = if self.progress.reputation[village] >= 0
                    && self.combat.hostility[village] > 0.
                {
                    if self.progress.coins >= 12 {
                        self.progress.coins -= 12;
                        Ok("Reparations paid. Your neighbors have accepted your apology.")
                    } else {
                        Err("You need 12 crowns to pay reparations.")
                    }
                } else {
                    self.progress.make_amends(village)
                };
                if result.is_ok() {
                    self.combat.hostility[village] = 0.;
                    for (i, npc) in self.world.npcs.iter().enumerate() {
                        if npc.home.distance([BRIARGLEN, WILLOWFORD][village]) < 320. {
                            self.combat.residents[i].health = crate::combat::RESIDENT_HEALTH;
                            self.combat.residents[i].down = 0.;
                            self.combat.residents[i].windup = 0.;
                        }
                    }
                }
                result
            }
            _ => return,
        };
        match result {
            Ok(message) => {
                self.message(message, Cue::Forge);
                self.sync_equipment();
                self.persist();
            }
            Err(message) => self.message(message, Cue::Talk),
        }
    }

    pub fn dodge(&mut self, aim: Option<Vec2>) {
        if self.panel != Panel::None || self.dialogue.is_some() {
            return;
        }
        let direction =
            aim.filter(|d| d.length_squared() > 0.01)
                .unwrap_or(match self.player.facing {
                    1 => Vec2::NEG_X,
                    2 => Vec2::X,
                    3 => Vec2::NEG_Y,
                    _ => Vec2::Y,
                });
        if self.combat.try_dodge(direction) {
            self.cues.push(Cue::Dodge);
        }
    }

    pub fn attack(&mut self, aim: Option<Vec2>) {
        if self.panel != Panel::None || self.dialogue.is_some() {
            return;
        }
        if let Some(direction) = aim.filter(|d| d.length_squared() > 1.) {
            self.player.facing = crate::world::facing(direction);
        }
        let kills = self.combat.kills;
        let before: Vec<_> = self.combat.residents.iter().map(|r| r.health).collect();
        let effects = self.combat.effects.len();
        if !self
            .combat
            .attack(&self.world, self.player.pos, self.player.facing)
        {
            return;
        }
        self.cues.push(Cue::Sword);
        if self.combat.effects.len() > effects {
            self.cues.push(Cue::Hit);
        }
        let mut harmed = [false; 2];
        for (i, old) in before.iter().enumerate() {
            if self.combat.residents[i].health < *old {
                let v = usize::from(
                    self.world.npcs[i].home.distance(WILLOWFORD)
                        < self.world.npcs[i].home.distance(BRIARGLEN),
                );
                harmed[v] = true;
            }
        }
        for (village, hit) in harmed.into_iter().enumerate() {
            if hit {
                self.progress.add_rep(village, -15);
                self.message(
                    format!(
                        "{} is hostile! Village trust −15. R to make amends.",
                        if village == 0 {
                            "Briarglen"
                        } else {
                            "Willowford"
                        }
                    ),
                    Cue::Hurt,
                );
                self.persist();
            }
        }
        if self.combat.kills > kills {
            self.progress.guardian_defeated = self.combat.guardian_defeated();
            self.message(
                if self.progress.guardian_defeated && !self.progress.relic_taken {
                    "The guardian falls · E to take the Emberheart from its altar"
                } else {
                    "Monster defeated · Collect its spoils"
                },
                Cue::Quest,
            );
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
        self.running = false;
        if self.panel != Panel::None || self.dialogue.is_some() {
            return;
        }
        self.elapsed += dt;
        self.save_time += dt;
        let speed = self
            .combat
            .movement_speed(running, direction.length_squared() > 0., dt);
        self.running = speed > 100.;
        if direction.length_squared() > 0. && self.combat.dodge <= 0. {
            let dir = direction.normalize();
            self.player.facing = if dir.x.abs() > dir.y.abs() {
                if dir.x < 0. { 1 } else { 2 }
            } else if dir.y < 0. {
                3
            } else {
                0
            };
            let next = self.world.slide_move(self.player.pos, dir * dt * speed);
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
        let health = self.combat.health;
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
            self.cues.push(Cue::Defeat);
            self.camera = self.player.pos - Vec2::new(0., 24.);
            self.toast = format!(
                "Rescued to {} · Your belongings are safe",
                if willow { "Willowford" } else { "Briarglen" }
            );
            self.toast_time = 7.;
            self.persist();
        }
        if self.combat.health < health {
            self.cues.push(Cue::Hurt);
        }
        let (coins, materials) = self.combat.take_loot(self.player.pos);
        if coins > 0 || materials > 0 {
            self.progress.award_loot(coins, materials);
            self.message(
                format!("Collected {coins} crowns · {materials} iron shards"),
                Cue::Loot,
            );
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
            progress: self.progress.clone(),
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
        match self.progress.forge {
            ForgeQuest::Seeking => "Find the Emberheart in Emberwatch Ruins, northeast.",
            ForgeQuest::Recovered => "Bring the Emberheart to Alden in Briarglen.",
            ForgeQuest::NotStarted if self.quest == Quest::Delivered => {
                "Speak with Alden at the Briarglen forge."
            }
            _ => self.quest.objective(),
        }
    }
    pub fn rain(&self) -> f32 {
        let t = self.elapsed.rem_euclid(300.);
        ((t - 110.) / 20.).clamp(0., 1.) * ((235. - t) / 25.).clamp(0., 1.)
    }
    pub fn night(&self) -> f32 {
        ((self.elapsed / 120. - 1.2).sin() * 0.8).clamp(0., 0.65)
    }
    pub fn ambience(&self) -> Ambience {
        let outdoors = if self.world.region(self.player.pos) == crate::world::DUNGEON {
            0.25
        } else {
            1.
        };
        Ambience {
            walking: self.player.walking && self.panel == Panel::None && self.dialogue.is_none(),
            running: self.running,
            forest: if self.world.region(self.player.pos).contains("wood") {
                1.
            } else {
                0.3
            },
            river: (1.
                - (self.player.pos.x - crate::world::river_x(self.player.pos.y)).abs() / 150.)
                .clamp(0., 1.),
            rain: self.rain() * outdoors,
            night: self.night(),
        }
    }
    pub fn interaction_prompt(&self) -> Option<&'static str> {
        use crate::world::{RELIC_POS, SECRET_CHEST, SECRET_SWITCH};
        if !self.progress.secret_open && self.player.pos.distance(SECRET_SWITCH) < 30. {
            Some("E  Examine engraved stone")
        } else if self.progress.secret_open
            && !self.progress.chest_taken
            && self.player.pos.distance(SECRET_CHEST) < 28.
        {
            Some("E  Open hidden cache")
        } else if !self.progress.relic_taken && self.player.pos.distance(RELIC_POS) < 28. {
            Some("E  Take the Emberheart")
        } else {
            None
        }
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
        // Interacting in a village without an available conversation now opens
        // its council; close that pause panel before checking retaliation AI.
        assert!(game.panel == Panel::Village);
        game.toggle(Panel::Village);
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

    #[test]
    fn emberheart_journey_connects_forge_secret_guardian_and_unique_reward() {
        use crate::world::{BLACKSMITH, RELIC_POS, SECRET_CHEST, SECRET_SWITCH};

        let mut game = game();
        let alden = game.world.npcs[BLACKSMITH].pos + Vec2::new(0., 12.);
        assert!(game.world.can_walk(alden, 5.));
        game.player.pos = alden;
        game.interact();
        assert_eq!(game.progress.forge, ForgeQuest::Seeking);
        assert_eq!(game.dialogue.as_ref().unwrap().speaker, "Alden");
        game.interact();
        assert!(game.panel == Panel::Forge);
        game.progress.award_loot(36, 4);
        game.menu_action(1);
        assert_eq!((game.progress.coins, game.progress.materials), (0, 0));
        assert_eq!(game.combat.damage, 35.);
        game.progress.award_loot(28, 3);
        game.menu_action(2);
        assert_eq!((game.progress.coins, game.progress.materials), (0, 0));
        assert_eq!(game.combat.armor_multiplier, 0.8);
        game.toggle(Panel::Forge);

        game.player.pos = SECRET_SWITCH - Vec2::new(14., 0.);
        assert!(game.world.can_walk(game.player.pos, 5.));
        game.interact();
        assert!(game.progress.secret_open);
        game.player.pos = SECRET_CHEST;
        game.interact();
        assert!(game.progress.chest_taken);
        assert!(game.progress.coins > 0 && game.progress.materials > 0);
        let cache = game.progress.clone();
        game.interact();
        assert_eq!(
            game.progress, cache,
            "the opened cache cannot be looted twice"
        );

        game.player.pos = RELIC_POS + Vec2::new(0., 20.);
        assert!(game.world.can_walk(game.player.pos, 5.));
        game.interact();
        assert!(!game.progress.relic_taken);
        assert_eq!(game.progress.forge, ForgeQuest::Seeking);
        game.progress.guardian_defeated = true;
        game.combat.restore_guardian(true);
        game.interact();
        assert!(game.progress.relic_taken);
        assert_eq!(game.progress.forge, ForgeQuest::Recovered);

        game.player.pos = alden;
        let coins = game.progress.coins;
        game.interact();
        assert_eq!(game.progress.forge, ForgeQuest::Complete);
        assert_eq!(game.progress.coins, coins + 45);
        assert_eq!(game.combat.damage, 55.);
        assert_eq!(game.progress.reputation[0], 30);
        let reward = game.progress.clone();
        game.interact();
        game.toggle(Panel::Forge);
        game.interact();
        assert_eq!(game.progress, reward, "Alden's reward is awarded once");
    }

    #[test]
    fn reparations_require_payment_then_restore_village_conversations() {
        let mut game = game();
        let mira = game.world.npcs[0].pos;
        game.attack(Some(mira - game.player.pos));
        assert_eq!(game.progress.reputation, [-15, 0]);
        assert_eq!(game.combat.hostility, [60., 0.]);
        assert!(game.nearest_npc().is_none());
        game.village_panel();
        assert!(game.panel == Panel::Village);
        let debt = game.progress.amends_cost(0).unwrap();
        game.menu_action(1);
        assert_eq!(game.progress.coins, 0);
        assert_eq!(game.progress.reputation, [-15, 0]);
        assert_eq!(game.combat.hostility, [60., 0.]);

        game.progress.award_loot(debt + 7, 0);
        game.menu_action(1);
        assert_eq!(game.progress.coins, 7);
        assert_eq!(game.progress.reputation, [0, 0]);
        assert_eq!(game.combat.hostility, [0., 0.]);
        for index in 0..4 {
            assert!(!game.combat.resident_hostile(index, &game.world));
            assert_eq!(
                game.combat.residents[index].health,
                crate::combat::RESIDENT_HEALTH
            );
        }
        game.toggle(Panel::Village);
        game.interact();
        assert_eq!(game.dialogue.as_ref().unwrap().speaker, "Mira");
    }
}
