//! Persistent rewards and the blacksmith's Emberheart commission.
use crate::world::Dialogue;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ForgeQuest {
    #[default]
    NotStarted,
    Seeking,
    Recovered,
    Complete,
}

impl ForgeQuest {
    pub fn objective(self) -> &'static str {
        match self {
            Self::NotStarted => "Visit Alden, Briarglen's blacksmith",
            Self::Seeking => "Recover the Emberheart from Emberwatch Ruins",
            Self::Recovered => "Return the Emberheart to Alden in Briarglen",
            Self::Complete => "Emberbrand restored · the vale remembers your courage",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Progress {
    pub coins: u32,
    pub materials: u32,
    pub blade: u8,
    pub armor: u8,
    pub forge: ForgeQuest,
    pub reputation: [i16; 2],
    pub secret_open: bool,
    pub chest_taken: bool,
    pub relic_taken: bool,
    pub guardian_defeated: bool,
}

impl Progress {
    pub fn blade_damage(&self) -> f32 {
        [25.0, 35.0, 45.0, 55.0][self.blade.min(3) as usize]
    }

    pub fn armor_multiplier(&self) -> f32 {
        [1.0, 0.8, 0.6][self.armor.min(2) as usize]
    }

    pub fn blade_name(&self) -> &'static str {
        ["Iron sword", "Steel sword", "Tempered sword", "Emberbrand"][self.blade.min(3) as usize]
    }

    pub fn armor_name(&self) -> &'static str {
        ["Traveler's clothes", "Leather armor", "Brigandine"][self.armor.min(2) as usize]
    }

    pub fn reputation_name(&self, village: usize) -> &'static str {
        match self.reputation.get(village).copied().unwrap_or(0) {
            ..=-40 => "Feared",
            -39..=-1 => "Distrusted",
            0..=29 => "Welcomed",
            30..=69 => "Trusted",
            _ => "Beloved",
        }
    }

    /// Trusted neighbors receive a 20% coin discount; materials are unchanged.
    fn price(&self, coins: u32, materials: u32) -> (u32, u32) {
        let coins = if self.reputation[0] >= 30 {
            coins * 4 / 5
        } else {
            coins
        };
        (coins, materials)
    }

    pub fn blade_cost(&self) -> Option<(u32, u32)> {
        match self.blade {
            0 => Some(self.price(36, 4)),
            1 => Some(self.price(64, 8)),
            _ => None,
        }
    }

    pub fn armor_cost(&self) -> Option<(u32, u32)> {
        match self.armor {
            0 => Some(self.price(28, 3)),
            1 => Some(self.price(54, 7)),
            _ => None,
        }
    }

    fn pay(&mut self, cost: (u32, u32)) -> Result<(), &'static str> {
        if self.reputation[0] < 0 {
            return Err("Make amends with Briarglen before using Alden's forge.");
        }
        if self.coins < cost.0 || self.materials < cost.1 {
            return Err("Not enough coins or materials. Hunt creatures or search the ruins.");
        }
        self.coins -= cost.0;
        self.materials -= cost.1;
        Ok(())
    }

    pub fn upgrade_blade(&mut self) -> Result<&'static str, &'static str> {
        let cost = self
            .blade_cost()
            .ok_or("Your sword needs no further forging.")?;
        self.pay(cost)?;
        self.blade += 1;
        Ok(if self.blade == 1 {
            "Steel sword equipped · 35 damage."
        } else {
            "Tempered sword equipped · 45 damage."
        })
    }

    pub fn upgrade_armor(&mut self) -> Result<&'static str, &'static str> {
        let cost = self
            .armor_cost()
            .ok_or("You already wear Alden's finest armor.")?;
        self.pay(cost)?;
        self.armor += 1;
        Ok(if self.armor == 1 {
            "Leather armor equipped · 20% less damage."
        } else {
            "Brigandine equipped · 40% less damage."
        })
    }

    pub fn amends_cost(&self, village: usize) -> Option<u32> {
        let reputation = *self.reputation.get(village)?;
        (reputation < 0).then(|| 12 + u32::from(reputation.unsigned_abs()) / 2)
    }

    pub fn make_amends(&mut self, village: usize) -> Result<&'static str, &'static str> {
        let cost = self
            .amends_cost(village)
            .ok_or("You are already welcome in this village.")?;
        if self.coins < cost {
            return Err("You need more coins to pay reparations to this village.");
        }
        self.coins -= cost;
        self.reputation[village] = 0;
        Ok("Reparations paid. The village has accepted your apology.")
    }

    pub fn add_rep(&mut self, village: usize, amount: i16) {
        if let Some(reputation) = self.reputation.get_mut(village) {
            *reputation = reputation.saturating_add(amount).clamp(-100, 100);
        }
    }

    pub fn award_loot(&mut self, coins: u32, materials: u32) {
        self.coins = self.coins.saturating_add(coins);
        self.materials = self.materials.saturating_add(materials);
    }

    pub fn talk_blacksmith(&mut self) -> Dialogue {
        if self.reputation[0] < 0 {
            return Dialogue {
                speaker: "Alden",
                title: "A debt to the village",
                text: "A forge protects its neighbors. Make amends for the harm you caused, and we can speak of swords again.",
            };
        }
        // Finding the relic before accepting the commission is valid exploration.
        if self.relic_taken && self.forge != ForgeQuest::Complete {
            self.forge = ForgeQuest::Recovered;
        }
        let (title, text) = match self.forge {
            ForgeQuest::NotStarted => {
                self.forge = ForgeQuest::Seeking;
                (
                    "The sleeping forge",
                    "My family's Emberheart was lost at Emberwatch Ruins, deep in Whisperwood northeast of Willowford. A stone guardian watches the old forge. Bring the Emberheart home, and I will forge you a blade worthy of it.\n\nGather coins and materials from creatures. I can improve your sword and armor before you go.",
                )
            }
            ForgeQuest::Seeking => (
                "An ember in the woods",
                "Cross the Mere into Willowford, then head northeast into Whisperwood. Enter Emberwatch through its southern arch. Defeat the guardian and search the northern pedestal.\n\nOne old wall bears a leaf engraving. My grandmother said it opened to a patient hand.",
            ),
            ForgeQuest::Recovered => {
                self.forge = ForgeQuest::Complete;
                self.relic_taken = true;
                self.blade = 3;
                self.award_loot(45, 0);
                self.add_rep(0, 30);
                (
                    "Emberbrand rekindled",
                    "The Emberheart! After all these years... Hold still, traveler. A little fire, a sure hammer, and the vale has a protector once more.\n\nEmberbrand equipped · 55 damage. You received 45 coins and earned Briarglen's trust.",
                )
            }
            ForgeQuest::Complete => (
                "A friend at the forge",
                "Emberbrand suits you, friend. Take good care of the vale. My forge is always open if your armor needs strengthening, and trusted neighbors pay a little less.",
            ),
        };
        Dialogue {
            speaker: "Alden",
            title,
            text,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commission_rewards_only_once_and_supports_early_exploration() {
        let mut progress = Progress::default();
        progress.talk_blacksmith();
        assert_eq!(progress.forge, ForgeQuest::Seeking);
        progress.talk_blacksmith();
        assert_eq!(progress.coins, 0);
        progress.relic_taken = true;
        progress.talk_blacksmith();
        assert_eq!(progress.forge, ForgeQuest::Complete);
        assert_eq!(
            (progress.coins, progress.blade, progress.reputation[0]),
            (45, 3, 30)
        );
        let rewarded = progress.clone();
        progress.talk_blacksmith();
        assert_eq!(progress, rewarded);
        let mut early = Progress {
            relic_taken: true,
            ..Progress::default()
        };
        early.talk_blacksmith();
        assert_eq!(early.forge, ForgeQuest::Complete);
    }

    #[test]
    fn equipment_purchase_is_transactional_and_never_downgrades_emberbrand() {
        let mut progress = Progress {
            coins: 100,
            materials: 2,
            ..Progress::default()
        };
        let before = progress.clone();
        assert!(progress.upgrade_blade().is_err());
        assert_eq!(progress, before);
        progress.materials = 20;
        progress.upgrade_blade().unwrap();
        assert_eq!(
            (progress.coins, progress.materials, progress.blade),
            (64, 16, 1)
        );
        progress.upgrade_blade().unwrap();
        assert_eq!(progress.blade_damage(), 45.0);
        progress.blade = 3;
        assert!(progress.upgrade_blade().is_err());
        assert_eq!(progress.blade_damage(), 55.0);
    }

    #[test]
    fn trust_discounts_goods_and_amends_restore_trade() {
        let mut progress = Progress {
            coins: 100,
            materials: 20,
            ..Progress::default()
        };
        progress.add_rep(0, 30);
        assert_eq!(progress.armor_cost(), Some((22, 3)));
        progress.upgrade_armor().unwrap();
        assert_eq!(progress.armor_multiplier(), 0.8);
        progress.add_rep(0, -70);
        let before = progress.clone();
        assert!(progress.upgrade_armor().is_err());
        assert_eq!(progress, before);
        assert_eq!(progress.amends_cost(0), Some(32));
        progress.make_amends(0).unwrap();
        assert_eq!((progress.coins, progress.reputation[0]), (46, 0));
        assert!(progress.make_amends(0).is_err());
        assert!(progress.make_amends(99).is_err());
    }

    #[test]
    fn reputation_and_loot_have_safe_bounds() {
        let mut progress = Progress::default();
        progress.add_rep(0, i16::MAX);
        progress.add_rep(0, i16::MAX);
        assert_eq!(progress.reputation[0], 100);
        progress.add_rep(1, i16::MIN);
        progress.add_rep(1, i16::MIN);
        assert_eq!(progress.reputation[1], -100);
        progress.award_loot(u32::MAX, u32::MAX);
        progress.award_loot(10, 10);
        assert_eq!((progress.coins, progress.materials), (u32::MAX, u32::MAX));
    }

    #[test]
    fn failed_reparations_and_hostile_dialogue_do_not_consume_or_reward() {
        let mut progress = Progress {
            reputation: [-80, 0],
            relic_taken: true,
            ..Progress::default()
        };
        let before = progress.clone();
        assert!(progress.make_amends(0).is_err());
        progress.talk_blacksmith();
        assert_eq!(progress, before);
    }
}
