#![forbid(unsafe_code)]
mod art;
mod audio;
mod combat;
mod game;
mod progress;
mod render;
mod save;
mod world;

use game::{Game, Panel};
use wgame::{
    Library, Result, Window,
    canvas::{Button, Event, Key},
    glam::Vec2,
};

#[wgame::window(title = "Moss & Mere | The Green March", logical_size = (1280.0, 800.0))]
async fn main(mut window: Window<'_>) -> Result<()> {
    let library = Library::new(window.graphics());
    let mut renderer = render::Renderer::new(library)?;
    #[cfg(not(target_arch = "wasm32"))]
    let args: Vec<String> = std::env::args().collect();
    #[cfg(target_arch = "wasm32")]
    let args: Vec<String> = Vec::new();
    let smoke = args.iter().any(|a| a == "--smoke");
    let screenshot = args
        .windows(2)
        .find(|a| a[0] == "--screenshot")
        .map(|a| a[1].clone());
    let preview = smoke || screenshot.is_some();
    let mut game = Game::new(preview || args.iter().any(|a| a == "--fresh"));
    game.save_enabled = !preview;
    if preview {
        match args
            .windows(2)
            .find(|a| a[0] == "--view")
            .map(|a| a[1].as_str())
        {
            Some("ruins") => {
                game.player.pos = world::GUARDIAN_HOME + Vec2::new(0., 65.);
                game.progress.forge = progress::ForgeQuest::Seeking;
                game.toast = "Emberwatch Ruins · The sleeping forge".into();
            }
            Some("rain") => {
                game.elapsed = 180.;
                game.player.pos = world::WILLOWFORD;
            }
            Some("forge") => {
                game.player.pos = game.world.npcs[world::BLACKSMITH].pos + Vec2::new(0., 20.);
                game.progress.coins = 80;
                game.progress.materials = 12;
                game.progress.talk_blacksmith();
                game.panel = Panel::Forge;
            }
            Some("river") => {
                game.player.pos = Vec2::new(world::river_x(world::BRIDGE_Y), world::BRIDGE_Y)
            }
            Some("forest") => game.player.pos = Vec2::new(470., 460.),
            Some("willowford") => game.player.pos = world::WILLOWFORD,
            Some("combat") => {
                if let Some(monster) = game.combat.monsters.first() {
                    let home = monster.pos;
                    for offset in [
                        Vec2::new(0., 30.),
                        Vec2::new(30., 0.),
                        Vec2::new(-30., 0.),
                        Vec2::new(0., -30.),
                    ] {
                        if game.world.can_walk(home + offset, world::PLAYER_RADIUS) {
                            game.player.pos = home + offset;
                            game.player.facing = world::facing(-offset);
                            break;
                        }
                    }
                    game.toast = "Wilderness encounter · Space / click to strike".into();
                }
            }
            _ => {}
        }
        game.camera = game.player.pos - Vec2::new(0., 24.);
    }
    if args.iter().any(|a| a == "--map") {
        game.panel = Panel::Map;
    }
    let mut audio = audio::Audio::new(!preview);
    let mut last = wgame::app::time::Instant::now();
    let mut frames = 0;
    'frames: while let Some(mut frame) = window.next_frame().await? {
        let now = wgame::app::time::Instant::now();
        let dt = (now - last).as_secs_f32().min(0.05);
        last = now;
        let size = render::DESIGN;
        let viewport =
            Vec2::new(frame.size().0 as f32, frame.size().1 as f32) / frame.scale_factor() as f32;
        let fit = (viewport / size).min_element();
        let input = frame.input();
        let down = |k| input.key_down(k);
        let direction = Vec2::new(
            (down(Key::Character('d')) || down(Key::ArrowRight)) as u8 as f32
                - (down(Key::Character('a')) || down(Key::ArrowLeft)) as u8 as f32,
            (down(Key::Character('s')) || down(Key::ArrowDown)) as u8 as f32
                - (down(Key::Character('w')) || down(Key::ArrowUp)) as u8 as f32,
        );
        let mut quit = false;
        for event in &frame.input().events {
            match *event {
                Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                } => match key {
                    Key::Character('e') | Key::Enter => game.interact(),
                    Key::Space => game.attack(None),
                    Key::Character('m') => game.toggle(Panel::Map),
                    Key::Character('j') | Key::Tab => game.toggle(Panel::Journal),
                    Key::Character('h') | Key::Character('?') => game.toggle(Panel::Help),
                    Key::Escape => {
                        if game.dialogue.is_some() {
                            game.dialogue = None;
                        } else if game.panel != Panel::None {
                            game.panel = Panel::None;
                        } else {
                            game.panel = Panel::Pause;
                        }
                    }
                    Key::Character('q') if game.panel == Panel::Pause => quit = game.persist(),
                    Key::Character('q') => game.dodge(Some(direction)),
                    Key::Character('r') => game.village_panel(),
                    Key::Character('u') => {
                        audio.set_muted(!audio.is_muted());
                        game.muted = audio.is_muted();
                        game.toast = if game.muted {
                            "Sound muted · U to restore"
                        } else {
                            "Sound on · U to mute"
                        }
                        .into();
                        game.toast_time = 3.;
                    }
                    Key::Character('1') => game.menu_action(1),
                    Key::Character('2') => game.menu_action(2),
                    Key::Plus | Key::Character('=') => game.zoom = (game.zoom + 0.25).min(3.5),
                    Key::Minus => game.zoom = (game.zoom - 0.25).max(1.5),
                    _ => {}
                },
                Event::Button {
                    button: Button::Primary,
                    pressed: true,
                    position,
                } => {
                    let position = (position - (viewport - size * fit) * 0.5) / fit;
                    if position.y < 66. {
                        if position.x > size.x - 80. {
                            game.toggle(Panel::Help);
                        } else if position.x > size.x - 190. {
                            game.toggle(Panel::Journal);
                        } else if position.x > size.x - 280. {
                            game.toggle(Panel::Map);
                        }
                    } else if matches!(game.panel, Panel::Forge | Panel::Village)
                        && render::menu_hit(position).is_some()
                    {
                        game.menu_action(render::menu_hit(position).unwrap());
                    } else if game.dialogue.is_some() && position.y > size.y - 370. {
                        game.interact();
                    } else if game.panel != Panel::None && render::close_hit(position, size) {
                        game.panel = Panel::None;
                    } else if position.y >= 72.
                        && position.y < size.y - 44.
                        && position.x >= 0.
                        && position.x < size.x
                    {
                        let point = render::world_at(position, &game);
                        game.attack(Some(point - game.player.pos));
                    }
                }
                Event::Button {
                    button: Button::Secondary,
                    pressed: true,
                    position,
                } => {
                    let position = (position - (viewport - size * fit) * 0.5) / fit;
                    game.dodge(Some(render::world_at(position, &game) - game.player.pos));
                }
                Event::Scroll(scroll) => {
                    game.zoom = (game.zoom + scroll.y.signum() * 0.25).clamp(1.5, 3.5)
                }
                _ => {}
            }
        }
        if quit {
            frame.discard();
            break 'frames;
        }
        game.update(dt, direction, input.modifiers.shift);
        if preview && frames == 9 && args.iter().any(|a| a == "combat") {
            game.attack(None);
        }
        for cue in game.cues.drain(..) {
            audio.play(cue);
        }
        audio.update(dt, game.ambience());
        renderer.draw(&mut frame, &game);
        frame.present();
        frames += 1;
        if preview && frames == 12 {
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(path) = &screenshot {
                renderer.screenshot(&game, path).await?;
            }
            break;
        }
    }
    if !preview {
        game.persist();
    }
    Ok(())
}
