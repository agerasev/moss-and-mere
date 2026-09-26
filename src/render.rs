use crate::{
    art::{self, PixelArt},
    combat::{self, Kind, MAX_HEALTH, RESIDENT_HEALTH, RESPAWN_TIME, SWING_DURATION},
    game::{Game, Panel},
    progress::ForgeQuest,
    world::{BRIARGLEN, HEIGHT, PropKind, Quest, TILE, Tile, WIDTH, WILLOWFORD},
};
use std::collections::BTreeMap;
use wgame::{
    Library, Result,
    gfx::{Scene, Target, types::Color},
    glam::{Affine2, Vec2, Vec4},
    image::Image,
    prelude::*,
    texture::{Texture, TextureSettings},
    typography::{Font, FontData, FontTexture, TextAlign},
};

fn color(hex: u32) -> Vec4 {
    Vec4::new(
        ((hex >> 16) & 255) as f32 / 255.,
        ((hex >> 8) & 255) as f32 / 255.,
        (hex & 255) as f32 / 255.,
        1.,
    )
}
pub const DESIGN: Vec2 = Vec2::new(1280., 800.);
const INK: u32 = 0x172d29;
const PAPER: u32 = 0xece4ca;
const GOLD: u32 = 0xd5b575;
const MUTED: u32 = 0xabb7a0;

struct Sprite {
    texture: Texture,
    size: Vec2,
}
impl Sprite {
    fn new(library: &Library, art: PixelArt) -> Self {
        let data = art
            .pixels
            .iter()
            .map(|p| {
                Vec4::new(
                    p[0] as f32 / 255.,
                    p[1] as f32 / 255.,
                    p[2] as f32 / 255.,
                    p[3] as f32 / 255.,
                )
                .to_rgba_f16()
            })
            .collect::<Vec<_>>();
        Self {
            texture: library.make_texture(
                &Image::with_data((art.width, art.height), data),
                TextureSettings::nearest(),
            ),
            size: Vec2::new(art.width as f32, art.height as f32),
        }
    }
}

pub struct Renderer {
    library: Library,
    terrain: Vec<Sprite>,
    props: Vec<Sprite>,
    people: Vec<Sprite>,
    monsters: Vec<Sprite>,
    fonts: [Font; 2],
    rasters: BTreeMap<(usize, u32), FontTexture>,
    dpi: f32,
}

impl Renderer {
    pub fn new(library: Library) -> Result<Self> {
        let fonts = [
            library.make_font(&FontData::new(
                include_bytes!("../assets/DejaVuSans.ttf").to_vec(),
                0,
            )?),
            library.make_font(&FontData::new(
                include_bytes!("../assets/DejaVuSerif.ttf").to_vec(),
                0,
            )?),
        ];
        let terrain = (0..8)
            .flat_map(|k| (0..4).map(move |v| (k, v)))
            .map(|(k, v)| Sprite::new(&library, art::terrain(k, v)))
            .collect();
        let props = (0..14)
            .flat_map(|k| (0..4).map(move |v| (k, v)))
            .map(|(k, v)| Sprite::new(&library, art::prop(k, v)))
            .collect();
        let people = (0..9)
            .flat_map(|v| (0..4).flat_map(move |f| (0..3).map(move |s| (v, f, s))))
            .map(|(v, f, s)| Sprite::new(&library, art::person(v, f, s)))
            .collect();
        let monsters = (0..3)
            .flat_map(|k| (0..4).flat_map(move |f| (0..3).map(move |step| (k, f, step))))
            .map(|(k, f, step)| Sprite::new(&library, art::monster(k, f, step)))
            .collect();
        Ok(Self {
            library,
            terrain,
            props,
            people,
            monsters,
            fonts,
            rasters: BTreeMap::new(),
            dpi: 1.,
        })
    }
    fn rect(&self, scene: &mut Scene, p: Vec2, size: Vec2, c: u32) {
        self.rgba(scene, p, size, color(c));
    }
    fn rgba(&self, scene: &mut Scene, p: Vec2, size: Vec2, c: Vec4) {
        scene.add(&self.library.shapes().rectangle((p, p + size)).fill_color(c));
    }
    fn line(&self, scene: &mut Scene, a: Vec2, b: Vec2, width: f32, c: u32) {
        scene.add(&self.library.shapes().line(a, b, width).fill_color(color(c)));
    }
    fn circle(&self, scene: &mut Scene, p: Vec2, r: f32, c: u32) {
        scene.add(
            &self
                .library
                .shapes()
                .unit_circle()
                .fill_color(color(c))
                .scale(r)
                .move_to(p),
        );
    }
    #[allow(clippy::too_many_arguments)]
    fn label(
        &mut self,
        scene: &mut Scene,
        text: &str,
        p: Vec2,
        size: f32,
        c: u32,
        serif: bool,
        align: TextAlign,
    ) {
        let index = usize::from(serif);
        let key = (index, (size * self.dpi).to_bits());
        let font = &self.fonts[index];
        let raster = self
            .rasters
            .entry(key)
            .or_insert_with(|| font.rasterize(size * self.dpi));
        scene.add(
            &raster
                .text(text)
                .scale(size)
                .align(align)
                .multiply_color(color(c))
                .move_to(p),
        );
    }
    fn text(&mut self, s: &mut Scene, t: &str, x: f32, y: f32, size: f32, c: u32) {
        self.label(s, t, Vec2::new(x, y), size, c, false, TextAlign::Left);
    }
    fn title(&mut self, s: &mut Scene, t: &str, x: f32, y: f32, size: f32, c: u32) {
        self.label(s, t, Vec2::new(x, y), size, c, true, TextAlign::Left);
    }
    fn center(&mut self, s: &mut Scene, t: &str, x: f32, y: f32, size: f32, c: u32) {
        self.label(s, t, Vec2::new(x, y), size, c, false, TextAlign::Center);
    }
    fn sprite(&self, scene: &mut Scene, sprite: &Sprite, position: Vec2, scale: f32, foot: bool) {
        let p = if foot {
            position - Vec2::new(sprite.size.x * 0.5, sprite.size.y) * scale
        } else {
            position
        };
        scene.add(
            &self
                .library
                .shapes()
                .rectangle((p, p + sprite.size * scale))
                .fill_texture(&sprite.texture),
        );
    }
    fn panel(&self, scene: &mut Scene, p: Vec2, size: Vec2, c: u32) {
        self.rgba(
            scene,
            p + Vec2::new(5., 7.),
            size,
            Vec4::new(0.02, 0.05, 0.04, 0.35),
        );
        self.rect(scene, p, size, c);
        self.rect(scene, p, Vec2::new(size.x, 2.), GOLD);
        self.rect(
            scene,
            p + Vec2::new(0., size.y - 1.),
            Vec2::new(size.x, 1.),
            0x59644a,
        );
    }
    fn paragraph(&mut self, s: &mut Scene, text: &str, p: Vec2, width: f32, size: f32, c: u32) {
        let max = (width / (size * 0.54)) as usize;
        let mut line = String::new();
        let mut y = p.y;
        for word in text.split_whitespace() {
            if line.chars().count() + word.chars().count() + 1 > max && !line.is_empty() {
                self.text(s, &line, p.x, y, size, c);
                line.clear();
                y += size * 1.6;
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        if !line.is_empty() {
            self.text(s, &line, p.x, y, size, c);
        }
    }
    pub fn draw(&mut self, target: &mut impl Target, game: &Game) {
        let size = DESIGN;
        let viewport = Vec2::new(target.size().0 as f32, target.size().1 as f32);
        let dpi = (viewport / size).min_element();
        if self.dpi != dpi {
            self.rasters.clear();
            self.dpi = dpi;
        }
        target.clear(color(INK));
        let camera = target
            .physical_camera()
            .transform(Affine2::from_scale_angle_translation(
                Vec2::splat(dpi),
                0.,
                (viewport - size * dpi) * 0.5,
            ));
        let mut scene = Scene::default();
        self.land(&mut scene, game, size);
        self.hud(&mut scene, game, size);
        if game.panel != Panel::None {
            self.overlay(&mut scene, game, size);
        }
        if game.dialogue.is_some() {
            self.dialogue(&mut scene, game, size);
        }
        if game.toast.starts_with("Could not save:") && game.toast_time > 0. {
            self.panel(&mut scene, Vec2::new(180., 78.), Vec2::new(920., 64.), INK);
            self.paragraph(
                &mut scene,
                &game.toast,
                Vec2::new(198., 103.),
                880.,
                13.,
                GOLD,
            );
        }
        target.render(&camera, &scene.bake());
    }
    fn land(&mut self, s: &mut Scene, g: &Game, size: Vec2) {
        let zoom = g.zoom;
        let origin = world_origin(g);
        let screen = |p: Vec2| (p * zoom + origin).round();
        let low = ((-origin) / zoom / TILE).floor().as_ivec2();
        let high = ((size - origin) / zoom / TILE).ceil().as_ivec2();
        for y in low.y.max(0)..high.y.min(HEIGHT as i32) {
            for x in low.x.max(0)..high.x.min(WIDTH as i32) {
                let tile = g.world.tile(x as usize, y as usize);
                let index = tile_index(tile);
                let hash = (x.wrapping_mul(374761393) ^ y.wrapping_mul(668265263)).unsigned_abs();
                let v = if tile == Tile::Water {
                    ((hash + (g.elapsed * 2.) as u32) % 4) as usize
                } else {
                    (hash % 4) as usize
                };
                self.sprite(
                    s,
                    &self.terrain[index * 4 + v],
                    screen(Vec2::new(x as f32 * TILE, y as f32 * TILE)),
                    zoom,
                    false,
                );
                if tile == Tile::Water && hash % 13 == 0 {
                    let p = screen(Vec2::new(
                        x as f32 * TILE + 3.,
                        y as f32 * TILE + 7. + (g.elapsed * 1.3 + hash as f32).sin() * 2.,
                    ));
                    self.rgba(
                        s,
                        p,
                        Vec2::new(5. * zoom, zoom),
                        Vec4::new(0.56, 0.79, 0.72, 0.45),
                    );
                }
            }
        }
        let on_screen = |p: Vec2| {
            let p = screen(p);
            p.x > -200. && p.x < size.x + 200. && p.y > 0. && p.y < size.y + 200.
        };
        // Ground decorations precede the painter-sorted solid sprites.
        for prop in &g.world.props {
            if on_screen(prop.pos) && matches!(prop.kind, PropKind::Flowers) {
                self.sprite(
                    s,
                    &self.props[prop_index(prop.kind) * 4 + prop.variant as usize % 4],
                    screen(prop.pos),
                    zoom,
                    true,
                );
            }
        }
        let mut objects = Vec::new();
        for (i, prop) in g.world.props.iter().enumerate() {
            if on_screen(prop.pos) && !matches!(prop.kind, PropKind::Flowers) {
                objects.push((prop.pos.y, 0, i));
            }
        }
        for (i, npc) in g.world.npcs.iter().enumerate() {
            if on_screen(npc.pos) {
                objects.push((npc.pos.y, 1, i));
            }
        }
        for (i, monster) in g.combat.monsters.iter().enumerate() {
            if on_screen(monster.pos)
                && (monster.health > 0. || monster.respawn > RESPAWN_TIME - 0.6)
            {
                objects.push((monster.pos.y, 3, i));
            }
        }
        for (i, pos) in [crate::world::SECRET_CHEST, crate::world::RELIC_POS]
            .into_iter()
            .enumerate()
        {
            if on_screen(pos) {
                objects.push((pos.y, 4, i));
            }
        }
        for drop in &g.combat.loot {
            if on_screen(drop.pos) {
                let p = screen(drop.pos);
                scene_ring(&self.library, s, p, Vec2::new(7., 3.) * zoom, color(GOLD));
                self.rect(
                    s,
                    p - Vec2::new(2., 6.) * zoom,
                    Vec2::new(5., 5.) * zoom,
                    0xdeb55e,
                );
                self.rect(
                    s,
                    p - Vec2::new(1., 6.) * zoom,
                    Vec2::new(2., 2.) * zoom,
                    0xffe9a3,
                );
            }
        }
        objects.push((g.player.pos.y, 2, 0));
        objects.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (_, kind, i) in objects {
            match kind {
                0 => {
                    let prop = &g.world.props[i];
                    let sprite = &self.props[prop_index(prop.kind) * 4 + prop.variant as usize % 4];
                    let delta = prop.pos - g.player.pos;
                    // Fade foreground canopies and roofs so the traveller stays visible.
                    let occludes = matches!(
                        prop.kind,
                        PropKind::Tree
                            | PropKind::Pine
                            | PropKind::House
                            | PropKind::Wall
                            | PropKind::Gate
                    ) && delta.y > 0.
                        && delta.y < sprite.size.y + 8.
                        && delta.x.abs() < sprite.size.x * 0.5 + 6.;
                    let p = screen(prop.pos) - Vec2::new(sprite.size.x * 0.5, sprite.size.y) * zoom;
                    s.add(
                        &self
                            .library
                            .shapes()
                            .rectangle((p, p + sprite.size * zoom))
                            .fill_texture(&sprite.texture)
                            .multiply_color(Vec4::new(1., 1., 1., if occludes { 0.4 } else { 1. })),
                    );
                }
                1 => {
                    let npc = &g.world.npcs[i];
                    let resident = &g.combat.residents[i];
                    if resident.down > 0. {
                        continue;
                    }
                    let step = if npc.walking {
                        1 + ((g.elapsed * 6.) as usize + i) % 2
                    } else {
                        0
                    };
                    let sprite = &self.people
                        [(npc.variant as usize % 9) * 12 + npc.facing as usize * 3 + step];
                    let pos =
                        screen(npc.pos) - Vec2::new(sprite.size.x * 0.5, sprite.size.y) * zoom;
                    let tint = if resident.hit_flash > 0. {
                        Vec4::new(1., 0.45, 0.35, 1.)
                    } else {
                        Vec4::ONE
                    };
                    s.add(
                        &self
                            .library
                            .shapes()
                            .rectangle((pos, pos + sprite.size * zoom))
                            .fill_texture(&sprite.texture)
                            .multiply_color(tint),
                    );
                    if resident.windup > 0. {
                        scene_ring(
                            &self.library,
                            s,
                            screen(npc.pos),
                            Vec2::new(15., 6.) * zoom,
                            color(0xe77c57),
                        );
                    }
                }
                3 => {
                    let monster = &g.combat.monsters[i];
                    let step = if monster.walking {
                        1 + (g.elapsed * 7.) as usize % 2
                    } else {
                        0
                    };
                    let kind = match monster.kind {
                        Kind::Slime => 0,
                        Kind::Wolf => 1,
                        Kind::Guardian => 2,
                    };
                    let sprite = &self.monsters[kind * 12 + monster.facing as usize * 3 + step];
                    let pos =
                        screen(monster.pos) - Vec2::new(sprite.size.x * 0.5, sprite.size.y) * zoom;
                    let alpha = if monster.health <= 0. {
                        ((monster.respawn - (RESPAWN_TIME - 0.6)) / 0.6).clamp(0., 1.)
                    } else {
                        1.
                    };
                    let tint = if monster.hit_flash > 0. {
                        Vec4::new(1., 0.4, 0.3, alpha)
                    } else {
                        Vec4::new(1., 1., 1., alpha)
                    };
                    s.add(
                        &self
                            .library
                            .shapes()
                            .rectangle((pos, pos + sprite.size * zoom))
                            .fill_texture(&sprite.texture)
                            .multiply_color(tint),
                    );
                    if monster.windup > 0. {
                        scene_ring(
                            &self.library,
                            s,
                            screen(monster.pos),
                            Vec2::new(
                                if monster.kind == Kind::Guardian {
                                    32.
                                } else {
                                    19.
                                },
                                if monster.kind == Kind::Guardian {
                                    11.
                                } else {
                                    7.
                                },
                            ) * zoom,
                            color(0xef8760),
                        );
                    }
                }
                4 => {
                    let (pos, kind, variant) = if i == 0 {
                        (
                            crate::world::SECRET_CHEST,
                            12,
                            usize::from(g.progress.chest_taken),
                        )
                    } else {
                        (
                            crate::world::RELIC_POS,
                            13,
                            usize::from(g.progress.relic_taken),
                        )
                    };
                    self.sprite(s, &self.props[kind * 4 + variant], screen(pos), zoom, true);
                }
                _ => {
                    let p = screen(g.player.pos);
                    if g.combat.dodge > 0. {
                        for i in 1..4 {
                            scene_ring(
                                &self.library,
                                s,
                                p - g.combat.dodge_direction() * (i as f32 * 7. * zoom),
                                Vec2::new(8., 3.) * zoom,
                                Vec4::new(0.8, 0.9, 0.8, 0.3),
                            );
                        }
                    }
                    scene_ring(
                        &self.library,
                        s,
                        p - Vec2::new(0., 3. * zoom),
                        Vec2::new(9. * zoom, 3. * zoom),
                        color(GOLD),
                    );
                    let step = if g.player.walking {
                        1 + (g.elapsed * 9.) as usize % 2
                    } else {
                        0
                    };
                    if g.combat.invulnerable <= 0. || ((g.elapsed * 18.) as u32).is_multiple_of(2) {
                        self.sprite(
                            s,
                            &self.people[g.player.facing as usize * 3 + step],
                            p,
                            zoom,
                            true,
                        );
                    }
                }
            }
        }
        self.atmosphere(s, g, origin);
        self.combat_effects(s, g, origin);
        for (i, npc) in g.world.npcs.iter().enumerate() {
            if g.combat.resident_hostile(i, &g.world) || g.combat.residents[i].down > 0. {
                continue;
            }
            let quest_npc = (npc.name == "Mira" && g.quest == Quest::NotStarted)
                || (npc.name == "Rowan" && g.quest == Quest::Carrying)
                || (npc.name == "Alden"
                    && matches!(
                        g.progress.forge,
                        ForgeQuest::NotStarted | ForgeQuest::Recovered
                    ));
            if quest_npc && on_screen(npc.pos) {
                let p = screen(npc.pos) - Vec2::new(0., 34. * zoom + (g.elapsed * 2.).sin() * 3.);
                self.circle(s, p, 11., INK);
                self.center(s, "!", p.x, p.y + 5., 16., GOLD);
            }
        }
        // Occasional drifting motes make the still landscape feel inhabited.
        for i in 0..22 {
            let t = i as f32;
            let x = (t * 173.1 + g.elapsed * 4. + origin.x * 0.25).rem_euclid(size.x);
            let y =
                (t * 89.3 + (g.elapsed * 0.4 + t).sin() * 13. + origin.y * 0.2).rem_euclid(size.y);
            self.rgba(
                s,
                Vec2::new(x, y),
                Vec2::splat(if i % 3 == 0 { 3. } else { 2. }),
                Vec4::new(0.94, 0.87, 0.61, 0.32),
            );
        }
        if g.panel == Panel::None
            && g.dialogue.is_none()
            && let Some(index) = g.nearest_npc()
        {
            let npc = &g.world.npcs[index];
            let p = screen(npc.pos) + Vec2::new(0., 63.);
            let text = format!("E  Talk to {}", npc.name);
            let width = text.len() as f32 * 7. + 28.;
            self.panel(
                s,
                p - Vec2::new(width / 2., 22.),
                Vec2::new(width, 34.),
                INK,
            );
            self.center(s, &text, p.x, p.y, 13., PAPER);
        }
        if g.panel == Panel::None
            && g.dialogue.is_none()
            && let Some(text) = g.interaction_prompt()
        {
            let p = screen(g.player.pos) + Vec2::new(0., 65.);
            self.panel(s, p - Vec2::new(155., 25.), Vec2::new(310., 38.), INK);
            self.center(s, text, p.x, p.y, 13., GOLD);
        }
        if g.toast_time > 0. && g.dialogue.is_none() && g.panel == Panel::None {
            let w = (g.toast.len() as f32 * 7.4 + 56.).min(size.x - 80.);
            let p = Vec2::new((size.x - w) / 2., 92.);
            self.panel(s, p, Vec2::new(w, 42.), INK);
            self.center(s, &g.toast, size.x / 2., 119., 14., PAPER);
        }
    }
    fn atmosphere(&self, s: &mut Scene, g: &Game, origin: Vec2) {
        let rain = g.rain();
        let night = g.night();
        self.rgba(
            s,
            Vec2::new(0., 72.),
            DESIGN - Vec2::new(0., 116.),
            Vec4::new(0.08, 0.14, 0.22, night * 0.42 + rain * 0.13),
        );
        for prop in &g.world.props {
            if prop.kind == PropKind::Torch {
                let p = (prop.pos - Vec2::new(0., 18.)) * g.zoom + origin;
                if p.x > -60. && p.x < DESIGN.x + 60. && p.y > 20. && p.y < DESIGN.y {
                    for i in 0..3 {
                        s.add(
                            &self
                                .library
                                .shapes()
                                .unit_circle()
                                .fill_color(Vec4::new(1., 0.64, 0.20, 0.035))
                                .scale((24. - i as f32 * 5. + (g.elapsed * 6.).sin()) * g.zoom)
                                .move_to(p),
                        );
                    }
                }
            }
        }
        for i in 0..(rain * 100.) as u32 {
            let x =
                (i as f32 * 137.7 - g.elapsed * 95. + origin.x * 0.15).rem_euclid(DESIGN.x + 20.);
            let y = (i as f32 * 93.3 + g.elapsed * 380. + origin.y * 0.15)
                .rem_euclid(DESIGN.y - 116.)
                + 72.;
            s.add(
                &self
                    .library
                    .shapes()
                    .line(Vec2::new(x, y), Vec2::new(x - 5., y + 12.), 1.)
                    .fill_color(Vec4::new(0.77, 0.85, 0.85, 0.32)),
            );
        }
    }
    fn combat_effects(&mut self, s: &mut Scene, g: &Game, origin: Vec2) {
        let zoom = g.zoom;
        let screen = |pos: Vec2| (pos * zoom + origin).round();
        for monster in &g.combat.monsters {
            if monster.health <= 0. || monster.pos.distance(g.player.pos) > 220. {
                continue;
            }
            let p = screen(monster.pos)
                - Vec2::new(
                    26.,
                    if monster.kind == Kind::Guardian {
                        56. * zoom
                    } else {
                        35. * zoom
                    },
                );
            self.center(s, monster.kind.name(), p.x + 26., p.y - 7., 10., PAPER);
            self.rect(s, p, Vec2::new(52., 5.), INK);
            self.rect(
                s,
                p + Vec2::ONE,
                Vec2::new(50. * monster.health / monster.kind.max_health(), 3.),
                0xd78b62,
            );
        }
        for (i, npc) in g.world.npcs.iter().enumerate() {
            let resident = &g.combat.residents[i];
            if !g.combat.resident_hostile(i, &g.world) || npc.pos.distance(g.player.pos) > 250. {
                continue;
            }
            let p = screen(npc.pos) - Vec2::new(24., 35. * zoom);
            if resident.down > 0. {
                self.center(
                    s,
                    &format!("{} · recovering", npc.name),
                    p.x + 24.,
                    p.y + 30.,
                    10.,
                    MUTED,
                );
                continue;
            }
            self.center(
                s,
                &format!("{} !", npc.name),
                p.x + 24.,
                p.y - 7.,
                10.,
                0xffbc8a,
            );
            self.rect(s, p, Vec2::new(48., 5.), INK);
            self.rect(
                s,
                p + Vec2::ONE,
                Vec2::new(46. * (resident.health / RESIDENT_HEALTH).clamp(0., 1.), 3.),
                0xe17b58,
            );
        }
        if g.combat.swing > 0. {
            let progress = 1. - (g.combat.swing / SWING_DURATION).clamp(0., 1.);
            let angle = g.combat.attack_dir.y.atan2(g.combat.attack_dir.x);
            let center = screen(g.player.pos - Vec2::new(0., 12.));
            for i in 0..12 {
                let a = angle - 0.95 + progress * 1.9 - i as f32 * 0.065;
                let b = a + 0.07;
                let radius = 31. * zoom;
                let aa = center + Vec2::new(a.cos(), a.sin()) * radius;
                let bb = center + Vec2::new(b.cos(), b.sin()) * radius;
                s.add(
                    &self
                        .library
                        .shapes()
                        .line(aa, bb, 2.5 * zoom)
                        .fill_color(Vec4::new(0.95, 0.87, 0.61, (1. - i as f32 / 12.) * 0.7)),
                );
            }
            let a = angle - 0.95 + progress * 1.9;
            let dir = Vec2::new(a.cos(), a.sin());
            let side = Vec2::new(-dir.y, dir.x);
            self.line(
                s,
                center + dir * 7. * zoom,
                center + dir * 31. * zoom,
                4. * zoom,
                INK,
            );
            self.line(
                s,
                center + dir * 12. * zoom,
                center + dir * 31. * zoom,
                2. * zoom,
                0xf5eed4,
            );
            self.line(
                s,
                center + dir * 11. * zoom - side * 4. * zoom,
                center + dir * 11. * zoom + side * 4. * zoom,
                2. * zoom,
                GOLD,
            );
        }
        for effect in &g.combat.effects {
            let p = screen(effect.pos) - Vec2::new(0., 40. * zoom + (1. - effect.life) * 30.);
            self.center(
                s,
                &format!("−{}", effect.amount),
                p.x,
                p.y,
                17.,
                if effect.player { 0xff9d7c } else { 0xffedba },
            );
        }
    }
    fn hud(&mut self, s: &mut Scene, g: &Game, size: Vec2) {
        self.rect(s, Vec2::ZERO, Vec2::new(size.x, 72.), INK);
        self.rect(s, Vec2::new(0., 71.), Vec2::new(size.x, 1.), 0x68735b);
        self.line(s, Vec2::new(29., 29.), Vec2::new(41., 17.), 2., GOLD);
        self.line(s, Vec2::new(41., 17.), Vec2::new(53., 29.), 2., GOLD);
        self.line(s, Vec2::new(29., 29.), Vec2::new(41., 48.), 2., GOLD);
        self.line(s, Vec2::new(53., 29.), Vec2::new(41., 48.), 2., GOLD);
        self.line(s, Vec2::new(41., 17.), Vec2::new(41., 48.), 1., GOLD);
        self.title(s, "Moss & Mere", 68., 35., 25., PAPER);
        self.text(s, "T H E  G R E E N  M A R C H", 70., 55., 9., MUTED);
        self.line(s, Vec2::new(298., 21.), Vec2::new(298., 51.), 1., 0x4b5e50);
        if size.x > 1000. {
            self.text(s, "EXPLORING", 326., 28., 9., MUTED);
            self.title(s, g.world.region(g.player.pos), 326., 52., 19., PAPER);
        }
        let village = if g.player.pos.distance(BRIARGLEN) < g.player.pos.distance(WILLOWFORD) {
            0
        } else {
            1
        };
        let hostile = g.combat.hostility[village] > 0.
            && g.player.pos.distance([BRIARGLEN, WILLOWFORD][village]) < 430.;
        let haven = combat::is_safe(g.player.pos) && !hostile;
        self.text(s, "VITALITY", 605., 25., 9., MUTED);
        self.text(
            s,
            &format!("{} / 100", g.combat.health.ceil() as u32),
            733.,
            25.,
            11.,
            PAPER,
        );
        self.rect(s, Vec2::new(605., 33.), Vec2::new(177., 7.), 0x3b4b40);
        self.rect(
            s,
            Vec2::new(605., 33.),
            Vec2::new(177. * (g.combat.health / MAX_HEALTH).clamp(0., 1.), 7.),
            if g.combat.health < 30. {
                0xda785b
            } else {
                0x9ebd78
            },
        );
        self.rect(s, Vec2::new(605., 46.), Vec2::new(177., 5.), 0x3b4b40);
        self.rect(
            s,
            Vec2::new(605., 46.),
            Vec2::new(
                177. * (g.combat.stamina / crate::combat::MAX_STAMINA).clamp(0., 1.),
                5.,
            ),
            0xc4b270,
        );
        self.text(s, "STAMINA", 605., 64., 8., MUTED);
        self.text(
            s,
            &format!("{} crowns", g.progress.coins),
            818.,
            28.,
            12.,
            GOLD,
        );
        self.text(
            s,
            &format!("{} iron shards", g.progress.materials),
            818.,
            45.,
            10.,
            MUTED,
        );
        self.text(
            s,
            if hostile {
                "VILLAGE HOSTILE"
            } else if haven {
                "Village sanctuary"
            } else if g.rain() > 0.5 {
                "Rain over the March"
            } else if g.night() > 0.3 {
                "Evening in the March"
            } else {
                "Wilderness"
            },
            818.,
            61.,
            9.,
            if hostile { 0xed9470 } else { MUTED },
        );
        for (label, key, x, active) in [
            ("Map", "M", size.x - 268., g.panel == Panel::Map),
            ("Journal", "J", size.x - 173., g.panel == Panel::Journal),
            ("Help", "H", size.x - 62., g.panel == Panel::Help),
        ] {
            if active {
                self.rect(
                    s,
                    Vec2::new(x - 10., 16.),
                    Vec2::new(if label == "Journal" { 99. } else { 80. }, 42.),
                    0x344b3e,
                );
            }
            self.text(s, label, x, 41., 13., PAPER);
            self.text(
                s,
                key,
                x + if label == "Journal" { 61. } else { 37. },
                41.,
                10.,
                GOLD,
            );
        }
        self.rect(s, Vec2::new(0., size.y - 44.), Vec2::new(size.x, 44.), INK);
        self.rect(
            s,
            Vec2::new(0., size.y - 44.),
            Vec2::new(size.x, 1.),
            0x68735b,
        );
        self.text(s, "WASD / ARROWS", 25., size.y - 17., 10., GOLD);
        self.text(s, "walk", 124., size.y - 17., 11., MUTED);
        self.text(s, "SHIFT", 177., size.y - 17., 10., GOLD);
        self.text(s, "run", 218., size.y - 17., 11., MUTED);
        self.text(s, "E", 262., size.y - 17., 10., GOLD);
        self.text(s, "talk", 280., size.y - 17., 11., MUTED);
        self.text(s, "+ / −", 330., size.y - 17., 10., GOLD);
        self.text(s, "zoom", 372., size.y - 17., 11., MUTED);
        self.text(s, "SPACE / CLICK", 435., size.y - 17., 10., GOLD);
        self.text(s, "strike", 530., size.y - 17., 11., MUTED);
        self.text(s, "Q / RIGHT CLICK", 600., size.y - 17., 10., GOLD);
        self.text(s, "dodge", 708., size.y - 17., 11., MUTED);
        self.text(s, "R", 775., size.y - 17., 10., GOLD);
        self.text(s, "village", 794., size.y - 17., 11., MUTED);
        self.label(
            s,
            if g.muted {
                "U  Sound off"
            } else {
                "U  Sound on"
            },
            Vec2::new(size.x - 24., size.y - 17.),
            11.,
            MUTED,
            false,
            TextAlign::Right,
        );
        if g.panel == Panel::None && g.dialogue.is_none() {
            let p = Vec2::new(24., size.y - 146.);
            self.panel(s, p, Vec2::new(365., 78.), INK);
            self.text(s, "YOUR JOURNEY", p.x + 17., p.y + 24., 10., GOLD);
            self.paragraph(s, g.objective(), p + Vec2::new(17., 47.), 330., 13., PAPER);
            self.minimap(s, g, Vec2::new(size.x - 178., size.y - 203.));
        }
    }
    fn minimap(&mut self, s: &mut Scene, g: &Game, p: Vec2) {
        self.panel(s, p, Vec2::new(154., 138.), INK);
        let o = p + Vec2::new(9., 11.);
        let scale = Vec2::new(136. / WIDTH as f32, 105. / HEIGHT as f32);
        for y in (0..HEIGHT).step_by(4) {
            for x in (0..WIDTH).step_by(4) {
                self.rect(
                    s,
                    o + Vec2::new(x as f32, y as f32) * scale,
                    scale * 4.,
                    map_color(g.world.tile(x, y)),
                );
            }
        }
        for pos in [BRIARGLEN, WILLOWFORD] {
            self.rect(
                s,
                o + pos / TILE * scale - Vec2::splat(2.),
                Vec2::splat(4.),
                PAPER,
            );
        }
        for monster in &g.combat.monsters {
            if monster.health > 0. && monster.pos.distance(g.player.pos) < 280. {
                self.circle(s, o + monster.pos / TILE * scale, 1.7, 0xe78c62);
            }
        }
        self.circle(s, o + g.player.pos / TILE * scale, 3., GOLD);
        self.center(s, "GREEN MARCH    ·    M", p.x + 77., p.y + 129., 8., MUTED);
    }
    fn overlay(&mut self, s: &mut Scene, g: &Game, size: Vec2) {
        self.rgba(
            s,
            Vec2::new(0., 72.),
            Vec2::new(size.x, size.y - 116.),
            Vec4::new(0.025, 0.055, 0.04, 0.68),
        );
        let panel_size = Vec2::new((size.x - 120.).min(960.), (size.y - 180.).min(620.));
        let p = (size - panel_size) * 0.5;
        self.panel(s, p, panel_size, PAPER);
        self.rect(
            s,
            p + Vec2::splat(9.),
            panel_size - Vec2::splat(18.),
            0xe3dbbd,
        );
        self.rect(
            s,
            p + Vec2::splat(11.),
            panel_size - Vec2::splat(22.),
            PAPER,
        );
        self.text(
            s,
            "MOSS & MERE  /  FIELD NOTES",
            p.x + 32.,
            p.y + 32.,
            10.,
            0x6f795d,
        );
        let heading = match g.panel {
            Panel::Map => "The Green March",
            Panel::Journal => "A traveller's journal",
            Panel::Help => "The road is yours",
            Panel::Pause => "A moment of quiet",
            Panel::Forge => "Alden’s forge",
            Panel::Village => "The village council",
            Panel::None => "",
        };
        self.title(s, heading, p.x + 32., p.y + 72., 31., INK);
        self.text(s, "×", p.x + panel_size.x - 43., p.y + 40., 24., INK);
        self.line(
            s,
            p + Vec2::new(32., 90.),
            p + Vec2::new(panel_size.x - 32., 90.),
            1.,
            0xbab895,
        );
        match g.panel {
            Panel::Map => self.map(s, g, p, panel_size),
            Panel::Journal => self.journal(s, g, p, panel_size),
            Panel::Help => self.help(s, p, panel_size),
            Panel::Forge => self.forge(s, g, p),
            Panel::Village => self.village(s, g, p),
            Panel::Pause => {
                self.paragraph(s,"Rest a while beneath the boughs. Your journey is saved automatically as you explore.",p+Vec2::new(40.,145.),panel_size.x-80.,19.,INK);
                self.text(
                    s,
                    "ESC   Continue your journey",
                    p.x + 40.,
                    p.y + 260.,
                    18.,
                    INK,
                );
                self.text(s, "Q      Save and leave", p.x + 40., p.y + 309., 18., INK);
                self.text(s, "M      Open the map", p.x + 40., p.y + 358., 18., INK);
            }
            Panel::None => {}
        }
    }
    fn map(&mut self, s: &mut Scene, g: &Game, p: Vec2, size: Vec2) {
        let area = size - Vec2::new(80., 155.);
        let scale = (area / Vec2::new(WIDTH as f32, HEIGHT as f32)).min_element();
        let map_size = Vec2::new(WIDTH as f32, HEIGHT as f32) * scale;
        let o = p + Vec2::new((size.x - map_size.x) / 2., 105.);
        self.rect(s, o - Vec2::splat(3.), map_size + Vec2::splat(6.), 0x8a9572);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                self.rect(
                    s,
                    o + Vec2::new(x as f32, y as f32) * scale,
                    Vec2::splat(scale + 0.1),
                    map_color(g.world.tile(x, y)),
                );
            }
        }
        for prop in &g.world.props {
            if matches!(prop.kind, PropKind::House) {
                self.rect(
                    s,
                    o + prop.pos / TILE * scale - Vec2::splat(2.5),
                    Vec2::new(5., 4.),
                    0xeee2b8,
                );
            }
        }
        for (pos, name) in [
            (BRIARGLEN, "Briarglen"),
            (WILLOWFORD, "Willowford"),
            (crate::world::DUNGEON_ENTRY, "Emberwatch"),
        ] {
            let pin = o + pos / TILE * scale;
            self.circle(s, pin, 5., INK);
            self.circle(s, pin, 3., PAPER);
            self.rect(s, pin + Vec2::new(-55., 10.), Vec2::new(110., 24.), INK);
            self.center(s, name, pin.x, pin.y + 27., 13., PAPER);
        }
        self.label(
            s,
            "WHISPERWOOD",
            o + Vec2::new(map_size.x * 0.25, map_size.y * 0.21),
            12.,
            0xd2dcb6,
            true,
            TextAlign::Center,
        );
        self.label(
            s,
            "MOSSMEADOW",
            o + Vec2::new(map_size.x * 0.74, map_size.y * 0.80),
            12.,
            0xe6dfb6,
            true,
            TextAlign::Center,
        );
        for monster in &g.combat.monsters {
            if monster.health > 0. {
                self.circle(s, o + monster.pos / TILE * scale, 2.3, 0xd98258);
            }
        }
        let player = o + g.player.pos / TILE * scale;
        self.circle(s, player, 7., INK);
        self.circle(s, player, 4., GOLD);
        self.center(s, "N", o.x + map_size.x - 20., o.y + 24., 13., PAPER);
        self.line(
            s,
            o + Vec2::new(map_size.x - 20., 32.),
            o + Vec2::new(map_size.x - 20., 51.),
            1.,
            PAPER,
        );
        self.text(
            s,
            "● You    □ Villages    ● Monsters    ~ Mere",
            p.x + 35.,
            p.y + size.y - 20.,
            11.,
            INK,
        );
        self.label(
            s,
            "M / ESC to return",
            p + Vec2::new(size.x - 34., size.y - 20.),
            11.,
            0x737858,
            false,
            TextAlign::Right,
        );
    }
    fn journal(&mut self, s: &mut Scene, g: &Game, p: Vec2, _: Vec2) {
        let x = p.x + 40.;
        let y = p.y + 130.;
        let right = p.x + 520.;
        self.text(s, "A LITTLE KINDNESS", x, y, 10., 0x7c693e);
        self.title(
            s,
            if g.quest == Quest::Delivered {
                "A promise kept"
            } else {
                "A parcel of herbs"
            },
            x,
            y + 33.,
            24.,
            INK,
        );
        self.paragraph(s,match g.quest {
            Quest::NotStarted => "Mira, the herbalist in Briarglen, has an errand for a traveler. Speak with her beside the well.",
            Quest::Carrying => "Carry Mira's herbs east over the bridge to Rowan in Willowford. Reward: 30 crowns and the villages' gratitude.",
            Quest::Delivered => "Rowan received the herbs. You earned 30 crowns and the trust of both villages.",
        },Vec2::new(x,y+67.),410.,14.,INK);
        self.line(
            s,
            Vec2::new(x, y + 170.),
            Vec2::new(x + 400., y + 170.),
            1.,
            0xbab895,
        );
        self.text(s, "THE SLEEPING FORGE", x, y + 205., 10., 0x7c693e);
        self.title(s, "An ember in the woods", x, y + 239., 24., INK);
        self.paragraph(
            s,
            g.progress.forge.objective(),
            Vec2::new(x, y + 272.),
            400.,
            15.,
            INK,
        );
        self.paragraph(s,"Enter the ruins northeast of Willowford through the southern arch. Dodge the guardian's strike, then inspect the altar. Look for a leaf carved into the eastern wall.",Vec2::new(x,y+335.),410.,13.,0x647057);
        self.line(
            s,
            Vec2::new(right - 30., y),
            Vec2::new(right - 30., y + 410.),
            1.,
            0xbab895,
        );
        self.text(s, "SATCHEL & EQUIPMENT", right, y, 10., 0x7c693e);
        self.title(
            s,
            &format!("{} crowns", g.progress.coins),
            right,
            y + 35.,
            25.,
            INK,
        );
        self.text(
            s,
            &format!("{} iron shards", g.progress.materials),
            right,
            y + 64.,
            15.,
            INK,
        );
        self.text(s, g.progress.blade_name(), right, y + 108., 18., INK);
        self.text(
            s,
            &format!("{} damage per strike", g.progress.blade_damage() as u32),
            right,
            y + 134.,
            12.,
            0x647057,
        );
        self.text(s, g.progress.armor_name(), right, y + 171., 18., INK);
        self.text(
            s,
            &format!(
                "{}% damage reduction",
                ((1. - g.progress.armor_multiplier()) * 100.).round() as u32
            ),
            right,
            y + 197.,
            12.,
            0x647057,
        );
        self.text(s, "VILLAGE REPUTATION", right, y + 252., 10., 0x7c693e);
        for (i, name) in ["Briarglen", "Willowford"].into_iter().enumerate() {
            self.text(
                s,
                &format!(
                    "{name} · {} ({:+})",
                    g.progress.reputation_name(i),
                    g.progress.reputation[i]
                ),
                right,
                y + 285. + i as f32 * 28.,
                14.,
                INK,
            );
        }
        self.text(
            s,
            &format!("Monsters defeated: {}", g.combat.kills),
            right,
            y + 375.,
            14.,
            INK,
        );
        self.text(
            s,
            if g.progress.secret_open {
                "Emberwatch's secret found"
            } else {
                "A secret waits in Emberwatch"
            },
            right,
            y + 404.,
            12.,
            0x647057,
        );
    }
    fn menu_button(&mut self, s: &mut Scene, p: Vec2, key: &str, title: &str, detail: &str) {
        self.panel(s, p, Vec2::new(880., 110.), INK);
        self.rect(s, p + Vec2::new(20., 29.), Vec2::new(43., 43.), 0x344b3e);
        self.center(s, key, p.x + 42., p.y + 58., 22., GOLD);
        self.title(s, title, p.x + 85., p.y + 40., 23., PAPER);
        self.text(s, detail, p.x + 85., p.y + 74., 14., MUTED);
    }
    fn forge(&mut self, s: &mut Scene, g: &Game, p: Vec2) {
        self.text(
            s,
            &format!(
                "Your purse: {} crowns     Iron shards: {}     Briarglen: {}",
                g.progress.coins,
                g.progress.materials,
                g.progress.reputation_name(0)
            ),
            p.x + 40.,
            p.y + 133.,
            16.,
            INK,
        );
        let blade = match g.progress.blade_cost() {
            Some((c, m)) => {
                format!("{c} crowns + {m} iron shards · permanently increases sword damage")
            }
            None => "Your blade is fully forged.".into(),
        };
        let armor = match g.progress.armor_cost() {
            Some((c, m)) => {
                format!("{c} crowns + {m} iron shards · reduces incoming damage by another 20%")
            }
            None => "You wear the finest armor from this forge.".into(),
        };
        self.menu_button(
            s,
            p + Vec2::new(40., 185.),
            "1",
            &format!("Improve {}", g.progress.blade_name()),
            &blade,
        );
        self.menu_button(
            s,
            p + Vec2::new(40., 325.),
            "2",
            &format!("Improve {}", g.progress.armor_name()),
            &armor,
        );
        self.text(
            s,
            "Trusted neighbors receive a 20% coin discount. Upgrades are equipped immediately.",
            p.x + 40.,
            p.y + 477.,
            14.,
            0x647057,
        );
        self.text(
            s,
            "Hunt creatures for supplies. Return the Emberheart to earn Alden’s unique blade.",
            p.x + 40.,
            p.y + 507.,
            14.,
            0x647057,
        );
        if g.toast_time > 0. {
            self.paragraph(s, &g.toast, p + Vec2::new(40., 554.), 850., 14., 0x7c693e);
        }
    }
    fn village(&mut self, s: &mut Scene, g: &Game, p: Vec2) {
        let Some(v) = g.near_village() else {
            return;
        };
        let name = ["Briarglen", "Willowford"][v];
        self.text(
            s,
            &format!(
                "{name} · {} ({:+})     Purse: {} crowns",
                g.progress.reputation_name(v),
                g.progress.reputation[v],
                g.progress.coins
            ),
            p.x + 40.,
            p.y + 133.,
            16.,
            INK,
        );
        let detail = if let Some(cost) = g.amends_cost(v) {
            format!("{cost} crowns · ends retaliation and restores negative reputation to neutral")
        } else {
            "There is no debt to settle. Your neighbors welcome you.".into()
        };
        self.menu_button(s, p + Vec2::new(40., 185.), "1", "Make amends", &detail);
        self.paragraph(s,"Hurting a resident calls the whole village to their defense. Leaving the area for a minute lets tempers cool, but people remember. Reparations settle your debt. Helping with errands earns trust, and trusted friends receive better prices at Alden’s forge.",p+Vec2::new(40.,350.),860.,17.,INK);
        if g.toast_time > 0. {
            self.paragraph(s, &g.toast, p + Vec2::new(40., 535.), 850., 15., 0x7c693e);
        }
    }
    fn help(&mut self, s: &mut Scene, p: Vec2, _: Vec2) {
        for (i, (key, action)) in [
            ("WASD / Arrows · Shift", "Walk · Run using stamina"),
            ("Space / left click", "Strike; click to aim your sword"),
            (
                "Q / right click",
                "Dodge; aim with movement keys or the pointer",
            ),
            (
                "E / Enter",
                "Talk, inspect, collect relics, continue dialogue",
            ),
            (
                "1 / 2 · R",
                "Buy upgrades in the forge · Open village council",
            ),
            ("M · J / Tab", "Map · Journal, equipment, and reputation"),
            ("+ / − / Mouse wheel", "Adjust the view"),
            ("U · Escape", "Mute sound · Close panels or pause"),
        ]
        .into_iter()
        .enumerate()
        {
            self.text(
                s,
                key,
                p.x + 40.,
                p.y + 132. + i as f32 * 39.,
                14.,
                0x76643b,
            );
            self.text(s, action, p.x + 285., p.y + 132. + i as f32 * 39., 14., INK);
        }
        self.paragraph(s,"Striking, running, and dodging spend stamina; catch your breath to recover. Orange rings warn of attacks. Collect glowing drops for crowns and iron shards. Peaceful villages heal you. Defeat returns you home with your belongings. Q while paused saves and quits.",p+Vec2::new(40.,470.),880.,14.,0x647057);
    }
    fn dialogue(&mut self, s: &mut Scene, g: &Game, size: Vec2) {
        let Some(d) = &g.dialogue else {
            return;
        };
        let w = (size.x - 100.).min(900.);
        let p = Vec2::new((size.x - w) * 0.5, size.y - 370.);
        self.panel(s, p, Vec2::new(w, 300.), PAPER);
        self.rect(s, p + Vec2::new(12., 12.), Vec2::new(116., 276.), 0xd2d3af);
        if let Some(npc) = g.world.npcs.iter().find(|n| n.name == d.speaker) {
            self.sprite(
                s,
                &self.people[(npc.variant as usize % 9) * 12],
                p + Vec2::new(71., 201.),
                4.,
                true,
            );
        }
        self.title(s, d.speaker, p.x + 150., p.y + 42., 25., INK);
        let role = g
            .world
            .npcs
            .iter()
            .find(|n| n.name == d.speaker)
            .map(|n| n.role)
            .unwrap_or(d.title);
        self.text(s, role, p.x + 150., p.y + 65., 10., 0x7a795b);
        self.paragraph(s, d.text, p + Vec2::new(150., 99.), w - 185., 15., INK);
        self.label(
            s,
            "E / ENTER  Continue",
            p + Vec2::new(w - 26., 280.),
            11.,
            0x76643b,
            false,
            TextAlign::Right,
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn screenshot(&mut self, g: &Game, path: &str) -> Result<()> {
        let mut target = self
            .library
            .texturing()
            .render_texture((1280, 800), TextureSettings::nearest())?;
        self.draw(&mut target, g);
        use wgame::image::ImageReadExt;
        let image = target.readback().await?;
        std::fs::write(path, image.slice((.., ..)).encode("png")?)?;
        Ok(())
    }
}

fn tile_index(tile: Tile) -> usize {
    match tile {
        Tile::Grass => 0,
        Tile::Forest => 1,
        Tile::Path => 2,
        Tile::Water => 3,
        Tile::Sand => 4,
        Tile::Bridge => 5,
        Tile::Farmland => 6,
        Tile::Stone => 7,
    }
}
fn prop_index(kind: PropKind) -> usize {
    match kind {
        PropKind::Tree => 0,
        PropKind::Pine => 1,
        PropKind::House => 2,
        PropKind::Well => 3,
        PropKind::Sign => 4,
        PropKind::Rock => 5,
        PropKind::Flowers => 6,
        PropKind::Fence => 7,
        PropKind::Wall => 8,
        PropKind::Torch => 9,
        PropKind::Anvil => 10,
        PropKind::Gate => 11,
    }
}
fn map_color(tile: Tile) -> u32 {
    match tile {
        Tile::Grass => 0x89955e,
        Tile::Forest => 0x3e664c,
        Tile::Path => 0xc9af73,
        Tile::Water => 0x528e93,
        Tile::Sand => 0xbfc28c,
        Tile::Bridge => 0xb8965c,
        Tile::Farmland => 0xa18b54,
        Tile::Stone => 0x727f76,
    }
}
fn scene_ring(l: &Library, s: &mut Scene, p: Vec2, r: Vec2, c: Vec4) {
    s.add(
        &l.shapes()
            .unit_circle()
            .stroke_color(0.09, c)
            .transform(Affine2::from_scale(r))
            .move_to(p),
    );
}
pub fn close_hit(pos: Vec2, size: Vec2) -> bool {
    let panel_size = Vec2::new((size.x - 120.).min(960.), (size.y - 180.).min(620.));
    let p = (size - panel_size) * 0.5 + Vec2::new(panel_size.x - 60., 10.);
    pos.cmpge(p).all() && pos.cmple(p + Vec2::splat(50.)).all()
}

fn world_origin(game: &Game) -> Vec2 {
    let size = DESIGN;
    let camera = game.camera.clamp(
        size / (2. * game.zoom),
        Vec2::new(WIDTH as f32 * TILE, HEIGHT as f32 * TILE) - size / (2. * game.zoom),
    );
    (Vec2::new(size.x / 2., (size.y + 16.) / 2.) - camera * game.zoom).round()
}
pub fn world_at(screen: Vec2, game: &Game) -> Vec2 {
    (screen - world_origin(game)) / game.zoom
}

pub fn menu_hit(pos: Vec2) -> Option<u8> {
    if !(200. ..=1080.).contains(&pos.x) {
        return None;
    }
    if (275. ..=385.).contains(&pos.y) {
        Some(1)
    } else if (415. ..=525.).contains(&pos.y) {
        Some(2)
    } else {
        None
    }
}
