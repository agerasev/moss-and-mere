//! Small, versioned save files shared by native and browser builds.

use crate::{
    combat::MAX_HEALTH,
    progress::{ForgeQuest, Progress},
    world::{HEIGHT, Quest, TILE, WIDTH},
};
use wgame::glam::Vec2;

const VERSION: &str = "moss-and-mere-v3";
const COMBAT_VERSION: &str = "moss-and-mere-v2";
const LEGACY_VERSION: &str = "moss-and-mere-v1";
const MAX_SAVE_BYTES: usize = 512;

pub struct Save {
    pub pos: Vec2,
    pub quest: Quest,
    pub visited: u8,
    pub elapsed: f32,
    pub health: f32,
    pub kills: u32,
    /// Bit 0 is Briarglen; bit 1 is Willowford.
    pub hostile: u8,
    pub progress: Progress,
}

impl Save {
    /// Missing, unreadable, and invalid saves leave the game at its starting state.
    /// Loading never changes the stored file.
    pub fn load() -> Option<Self> {
        Self::parse(&storage::read()?)
    }

    pub fn store(&self) -> Result<(), String> {
        let text = self.encode()?;
        storage::write(&text)
    }

    fn valid(&self) -> bool {
        self.pos.is_finite()
            && self.pos.x >= 0.0
            && self.pos.y >= 0.0
            && self.pos.x < WIDTH as f32 * TILE
            && self.pos.y < HEIGHT as f32 * TILE
            && self.visited & !0b11 == 0
            && self.elapsed.is_finite()
            && self.elapsed >= 0.0
            && self.health.is_finite()
            && self.health > 0.0
            && self.health <= MAX_HEALTH
            && self.hostile & !0b11 == 0
            && self.progress.blade <= 3
            && self.progress.armor <= 2
            && self
                .progress
                .reputation
                .iter()
                .all(|r| (-100..=100).contains(r))
            && (!self.progress.chest_taken || self.progress.secret_open)
            && (!self.progress.relic_taken || self.progress.guardian_defeated)
            && (!matches!(
                self.progress.forge,
                ForgeQuest::Recovered | ForgeQuest::Complete
            ) || self.progress.relic_taken)
            && (self.progress.forge != ForgeQuest::Complete || self.progress.blade == 3)
    }

    fn encode(&self) -> Result<String, String> {
        if !self.valid() {
            return Err("The current game state cannot be saved.".into());
        }
        let quest = match self.quest {
            Quest::NotStarted => "not-started",
            Quest::Carrying => "carrying",
            Quest::Delivered => "delivered",
        };
        let p = &self.progress;
        let forge = match p.forge {
            ForgeQuest::NotStarted => 0,
            ForgeQuest::Seeking => 1,
            ForgeQuest::Recovered => 2,
            ForgeQuest::Complete => 3,
        };
        let dungeon = u8::from(p.secret_open)
            | (u8::from(p.chest_taken) << 1)
            | (u8::from(p.relic_taken) << 2)
            | (u8::from(p.guardian_defeated) << 3);
        Ok(format!(
            "{VERSION}\nposition {} {}\nquest {quest}\nvisited {}\nelapsed {}\nhealth {}\nkills {}\nhostile {}\ninventory {} {}\nequipment {} {}\nforge {forge}\nreputation {} {}\ndungeon {dungeon}\n",
            self.pos.x,
            self.pos.y,
            self.visited,
            self.elapsed,
            self.health,
            self.kills,
            self.hostile,
            p.coins,
            p.materials,
            p.blade,
            p.armor,
            p.reputation[0],
            p.reputation[1]
        ))
    }

    fn parse(text: &str) -> Option<Self> {
        if text.len() > MAX_SAVE_BYTES {
            return None;
        }
        let mut words = text.split_whitespace();
        let version = words.next()?;
        if !matches!(version, VERSION | COMBAT_VERSION | LEGACY_VERSION)
            || words.next()? != "position"
        {
            return None;
        }
        let pos = Vec2::new(words.next()?.parse().ok()?, words.next()?.parse().ok()?);
        if words.next()? != "quest" {
            return None;
        }
        let quest = match words.next()? {
            "not-started" => Quest::NotStarted,
            "carrying" => Quest::Carrying,
            "delivered" => Quest::Delivered,
            _ => return None,
        };
        if words.next()? != "visited" {
            return None;
        }
        let visited = words.next()?.parse().ok()?;
        if words.next()? != "elapsed" {
            return None;
        }
        let elapsed = words.next()?.parse().ok()?;
        let (health, kills, hostile) = if version != LEGACY_VERSION {
            if words.next()? != "health" {
                return None;
            }
            let health = words.next()?.parse().ok()?;
            if words.next()? != "kills" {
                return None;
            }
            let kills = words.next()?.parse().ok()?;
            if words.next()? != "hostile" {
                return None;
            }
            let hostile = words.next()?.parse().ok()?;
            (health, kills, hostile)
        } else {
            (MAX_HEALTH, 0, 0)
        };
        let mut progress = Progress::default();
        if version == VERSION {
            if words.next()? != "inventory" {
                return None;
            }
            progress.coins = words.next()?.parse().ok()?;
            progress.materials = words.next()?.parse().ok()?;
            if words.next()? != "equipment" {
                return None;
            }
            progress.blade = words.next()?.parse().ok()?;
            progress.armor = words.next()?.parse().ok()?;
            if words.next()? != "forge" {
                return None;
            }
            progress.forge = match words.next()? {
                "0" => ForgeQuest::NotStarted,
                "1" => ForgeQuest::Seeking,
                "2" => ForgeQuest::Recovered,
                "3" => ForgeQuest::Complete,
                _ => return None,
            };
            if words.next()? != "reputation" {
                return None;
            }
            progress.reputation = [words.next()?.parse().ok()?, words.next()?.parse().ok()?];
            if words.next()? != "dungeon" {
                return None;
            }
            let flags: u8 = words.next()?.parse().ok()?;
            if flags > 15 {
                return None;
            }
            progress.secret_open = flags & 1 != 0;
            progress.chest_taken = flags & 2 != 0;
            progress.relic_taken = flags & 4 != 0;
            progress.guardian_defeated = flags & 8 != 0;
        } else {
            for i in 0..2 {
                if hostile & (1 << i) != 0 {
                    progress.reputation[i] = -15;
                }
            }
        }
        let save = Self {
            pos,
            quest,
            visited,
            elapsed,
            health,
            kills,
            hostile,
            progress,
        };
        (words.next().is_none() && save.valid()).then_some(save)
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod storage {
    use super::{MAX_SAVE_BYTES, Save};
    use std::{
        fs::{self, OpenOptions},
        io::{self, Read, Seek, Write},
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    const MAX_BACKUP_BYTES: u64 = 8 * 1024 * 1024;

    fn path() -> Option<PathBuf> {
        let base = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .or_else(|| {
                std::env::var_os("HOME")
                    .filter(|home| !home.is_empty())
                    .map(|home| PathBuf::from(home).join(".local/share"))
            })?;
        Some(base.join("moss-and-mere/save.txt"))
    }

    pub fn read() -> Option<String> {
        let file = fs::File::open(path()?).ok()?;
        let mut text = String::new();
        file.take(MAX_SAVE_BYTES as u64 + 1)
            .read_to_string(&mut text)
            .ok()?;
        (text.len() <= MAX_SAVE_BYTES).then_some(text)
    }

    pub fn write(text: &str) -> Result<(), String> {
        let path = path().ok_or("No user data directory is available for saving.")?;
        write_at(&path, text)
    }

    fn write_at(path: &Path, text: &str) -> Result<(), String> {
        let directory = path.parent().ok_or("Invalid save directory.")?;
        fs::create_dir_all(directory)
            .map_err(|error| format!("Cannot create save folder: {error}"))?;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("Cannot prepare save: {error}"))?
            .as_nanos();
        preserve_invalid(path, stamp)
            .map_err(|error| format!("Cannot preserve the existing save: {error}"))?;
        let temporary = directory.join(format!(".save-{}-{stamp}.tmp", std::process::id()));
        let result = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            file.write_all(text.as_bytes())?;
            file.sync_all()?;
            drop(file);
            // Keeping the temporary file beside the save makes replacement atomic.
            fs::rename(&temporary, path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result.map_err(|error: std::io::Error| format!("Cannot write save: {error}"))
    }

    fn preserve_invalid(path: &Path, stamp: u128) -> io::Result<()> {
        let mut original = match fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };
        let metadata = original.metadata()?;
        if !metadata.is_file() || metadata.len() > MAX_BACKUP_BYTES {
            return Err(io::Error::other("existing save is not a file under 8 MiB"));
        }
        let mut bytes = Vec::new();
        Read::by_ref(&mut original)
            .take(MAX_SAVE_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        if std::str::from_utf8(&bytes)
            .ok()
            .and_then(Save::parse)
            .is_some()
        {
            return Ok(());
        }
        original.rewind()?;
        // Reserve a fresh name so later damaged saves never replace earlier backups.
        for index in 0..1000 {
            let name = if index == 0 {
                "save.invalid.txt".to_owned()
            } else {
                format!("save.invalid-{stamp}-{index}.txt")
            };
            let backup_path = path.with_file_name(name);
            let mut backup = match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&backup_path)
            {
                Ok(file) => file,
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            };
            let result = (|| {
                let copied = io::copy(
                    &mut Read::by_ref(&mut original).take(MAX_BACKUP_BYTES + 1),
                    &mut backup,
                )?;
                if copied > MAX_BACKUP_BYTES {
                    return Err(io::Error::other("existing save grew beyond 8 MiB"));
                }
                backup.sync_all()
            })();
            if result.is_err() {
                drop(backup);
                let _ = fs::remove_file(&backup_path);
            }
            return result;
        }
        Err(io::Error::other("no unused backup name is available"))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::sync::atomic::{AtomicU64, Ordering};

        struct TestDirectory(PathBuf);
        impl TestDirectory {
            fn new() -> Self {
                static SEQUENCE: AtomicU64 = AtomicU64::new(0);
                let stamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos();
                let path = std::env::temp_dir().join(format!(
                    "moss-save-test-{}-{stamp}-{}",
                    std::process::id(),
                    SEQUENCE.fetch_add(1, Ordering::Relaxed)
                ));
                fs::create_dir(&path).unwrap();
                Self(path)
            }
        }
        impl Drop for TestDirectory {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }

        #[test]
        fn damaged_saves_are_preserved_without_replacing_prior_backups() {
            let directory = TestDirectory::new();
            let path = directory.0.join("save.txt");
            let current = crate::save::tests::example(crate::world::Quest::Carrying)
                .encode()
                .unwrap();
            let damaged = [0xff, 0xfe, 0x00, b'x'];
            fs::write(&path, damaged).unwrap();
            write_at(&path, &current).unwrap();
            assert_eq!(
                fs::read(directory.0.join("save.invalid.txt")).unwrap(),
                damaged
            );
            assert_eq!(fs::read_to_string(&path).unwrap(), current);

            let too_long = vec![b'x'; MAX_SAVE_BYTES + 100];
            fs::write(&path, &too_long).unwrap();
            write_at(&path, &current).unwrap();
            let backups = fs::read_dir(&directory.0)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .filter(|entry| entry.file_name().unwrap() != "save.txt")
                .map(|entry| fs::read(entry).unwrap())
                .collect::<Vec<_>>();
            assert_eq!(backups.len(), 2);
            assert!(backups.contains(&damaged.to_vec()));
            assert!(backups.contains(&too_long));
            write_at(&path, &current).unwrap();
            assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 3);
        }

        #[test]
        fn refused_backup_leaves_existing_save_untouched() {
            let directory = TestDirectory::new();
            let path = directory.0.join("save.txt");
            let original = fs::File::create(&path).unwrap();
            original.set_len(MAX_BACKUP_BYTES + 1).unwrap();
            assert!(write_at(&path, "replacement").is_err());
            assert_eq!(fs::metadata(&path).unwrap().len(), MAX_BACKUP_BYTES + 1);
            assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 1);
        }

        #[test]
        fn upgrading_a_valid_legacy_save_does_not_create_an_invalid_backup() {
            let directory = TestDirectory::new();
            let path = directory.0.join("save.txt");
            let legacy = crate::save::tests::LEGACY_SAVE;
            fs::write(&path, legacy).unwrap();
            let upgraded = Save::parse(legacy).unwrap().encode().unwrap();
            write_at(&path, &upgraded).unwrap();
            assert_eq!(fs::read_to_string(&path).unwrap(), upgraded);
            assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 1);
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod storage {
    use super::Save;

    // Keep the original key so existing browser progress upgrades in place.
    const KEY: &str = "moss-and-mere.save.v1";

    pub fn read() -> Option<String> {
        web_sys::window()?
            .local_storage()
            .ok()??
            .get_item(KEY)
            .ok()?
    }

    pub fn write(text: &str) -> Result<(), String> {
        let storage = web_sys::window()
            .ok_or("No browser window is available.")?
            .local_storage()
            .map_err(|_| "The browser has blocked local saves.")?
            .ok_or("The browser does not support local saves.")?;
        if let Some(existing) = storage
            .get_item(KEY)
            .map_err(|_| "The browser could not read the existing save.")?
            .filter(|text| Save::parse(text).is_none())
        {
            let mut backed_up = false;
            for index in 0..1000 {
                let backup_key = format!("moss-and-mere.save.invalid.{index}");
                if storage
                    .get_item(&backup_key)
                    .map_err(|_| "The browser could not inspect save backups.")?
                    .is_none()
                {
                    storage
                        .set_item(&backup_key, &existing)
                        .map_err(|_| "The browser could not preserve the existing save.")?;
                    backed_up = true;
                    break;
                }
            }
            if !backed_up {
                return Err("No unused browser save backup is available.".into());
            }
        }
        storage
            .set_item(KEY, text)
            .map_err(|_| "The browser could not store your save.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) const LEGACY_SAVE: &str =
        "moss-and-mere-v1\nposition 80 104\nquest carrying\nvisited 3\nelapsed 125.75\n";

    pub(super) fn example(quest: Quest) -> Save {
        Save {
            pos: Vec2::new(TILE * 2.5, TILE * 3.25),
            quest,
            visited: 0b11,
            elapsed: 125.75,
            health: 72.5,
            kills: 19,
            hostile: 0b01,
            progress: Progress {
                coins: 54,
                materials: 7,
                blade: 1,
                armor: 2,
                forge: ForgeQuest::Recovered,
                reputation: [-20, 35],
                secret_open: true,
                chest_taken: true,
                relic_taken: true,
                guardian_defeated: true,
            },
        }
    }

    #[test]
    fn round_trip_preserves_position_progress_time_and_combat() {
        for quest in [Quest::NotStarted, Quest::Carrying, Quest::Delivered] {
            let original = example(quest);
            let encoded = original.encode().unwrap();
            let restored = Save::parse(&encoded).unwrap();
            assert_eq!(restored.pos, original.pos);
            assert_eq!(restored.visited, original.visited);
            assert_eq!(restored.elapsed, original.elapsed);
            assert_eq!(restored.health, original.health);
            assert_eq!(restored.kills, original.kills);
            assert_eq!(restored.hostile, original.hostile);
            assert_eq!(restored.encode().unwrap(), encoded);
        }
    }

    #[test]
    fn legacy_saves_start_with_full_health_no_kills_and_peaceful_villages() {
        let restored = Save::parse(LEGACY_SAVE).unwrap();
        assert_eq!(restored.pos, Vec2::new(80.0, 104.0));
        assert_eq!(restored.visited, 0b11);
        assert_eq!(restored.elapsed, 125.75);
        assert_eq!(restored.health, MAX_HEALTH);
        assert_eq!(restored.kills, 0);
        assert_eq!(restored.hostile, 0);
        assert!(restored.encode().unwrap().starts_with(VERSION));
        assert!(Save::parse(&format!("{LEGACY_SAVE}extra")).is_none());
        assert!(Save::parse(&format!("{LEGACY_SAVE}health 100\nkills 0\n")).is_none());
    }

    #[test]
    fn combat_saves_migrate_inventory_without_losing_progress() {
        let old = "moss-and-mere-v2\nposition 680 1016\nquest delivered\nvisited 3\nelapsed 240\nhealth 80\nkills 7\nhostile 2\n";
        let save = Save::parse(old).unwrap();
        assert_eq!(save.kills, 7);
        assert_eq!(save.progress.coins, 0);
        assert_eq!(save.progress.reputation, [0, -15]);
        assert_eq!(save.progress.forge, ForgeQuest::NotStarted);
        assert_eq!(Save::parse(&save.encode().unwrap()).unwrap().hostile, 2);
    }

    #[test]
    fn rejects_malformed_and_unknown_save_formats() {
        let valid = example(Quest::Carrying).encode().unwrap();
        for invalid in [
            String::new(),
            valid.replace(VERSION, "moss-and-mere-v99"),
            valid.replace("inventory 54", "inventory -1"),
            valid.replace("equipment 1 2", "equipment 4 2"),
            valid.replace("equipment 1 2", "equipment 1 3"),
            valid.replace("forge 2", "forge 9"),
            valid.replace("forge 2", "forge 3"),
            valid.replace("dungeon 15", "dungeon 11"),
            valid.replace("reputation -20 35", "reputation -101 35"),
            valid.replace("dungeon 15", "dungeon 16"),
            valid.replace("dungeon 15", "dungeon 4"),
            valid.replace("position", "location"),
            valid.replace("carrying", "invented"),
            valid.replace("visited 3", "visited 256"),
            valid.replace("elapsed 125.75", "elapsed words"),
            valid.replace("health 72.5", "health words"),
            valid.replace("health", "life"),
            valid.replace("kills", "defeated"),
            valid.replace("kills 19", "kills -1"),
            valid.replace("kills 19", "kills 4294967296"),
            valid.replace("kills 19", "kills 1.5"),
            valid.replace("kills 19\n", ""),
            valid.replace("health 72.5\n", ""),
            valid.replace("hostile 1\n", ""),
            valid.replace("hostile", "hostility"),
            valid.replace("hostile 1", "hostile -1"),
            valid.replace("hostile 1", "hostile 256"),
            valid.replace("hostile 1", "hostile 1.5"),
            format!("{valid}extra"),
            "x".repeat(MAX_SAVE_BYTES + 1),
        ] {
            assert!(Save::parse(&invalid).is_none(), "accepted {invalid:?}");
        }
    }

    #[test]
    fn rejects_out_of_bounds_and_non_finite_values() {
        for pos in [
            Vec2::new(-1.0, 0.0),
            Vec2::new(0.0, -1.0),
            Vec2::new(WIDTH as f32 * TILE, 0.0),
            Vec2::new(0.0, HEIGHT as f32 * TILE),
            Vec2::new(f32::NAN, 0.0),
            Vec2::new(0.0, f32::INFINITY),
        ] {
            let mut save = example(Quest::NotStarted);
            save.pos = pos;
            assert!(save.encode().is_err());
        }
        let valid = example(Quest::NotStarted).encode().unwrap();
        for elapsed in ["-1", "NaN", "inf", "-inf"] {
            assert!(Save::parse(&valid.replace("125.75", elapsed)).is_none());
        }
        assert!(Save::parse(&valid.replace("visited 3", "visited 4")).is_none());
        for health in ["0", "-1", "100.01", "NaN", "inf", "-inf"] {
            assert!(
                Save::parse(&valid.replace("health 72.5", &format!("health {health}"))).is_none()
            );
        }
        for health in [0.0, -1.0, MAX_HEALTH + 0.01, f32::NAN, f32::INFINITY] {
            let mut save = example(Quest::NotStarted);
            save.health = health;
            assert!(save.encode().is_err());
        }
    }

    #[test]
    fn accepts_health_and_kill_count_limits() {
        for health in [f32::MIN_POSITIVE, MAX_HEALTH] {
            for kills in [0, u32::MAX] {
                let mut save = example(Quest::Delivered);
                save.health = health;
                save.kills = kills;
                let restored = Save::parse(&save.encode().unwrap()).unwrap();
                assert_eq!(restored.health, health);
                assert_eq!(restored.kills, kills);
            }
        }
    }

    #[test]
    fn accepts_only_the_two_village_hostility_bits() {
        for hostile in 0..=u8::MAX {
            let mut save = example(Quest::Delivered);
            save.hostile = hostile;
            let serialized = example(Quest::Delivered)
                .encode()
                .unwrap()
                .replace("hostile 1", &format!("hostile {hostile}"));
            if hostile <= 0b11 {
                assert_eq!(
                    Save::parse(&save.encode().unwrap()).unwrap().hostile,
                    hostile
                );
                assert_eq!(Save::parse(&serialized).unwrap().hostile, hostile);
            } else {
                assert!(save.encode().is_err());
                assert!(Save::parse(&serialized).is_none());
            }
        }
    }
}
