//! Small procedural soundscape; no recordings, assets, or external processes.
//! A missing audio device simply leaves the game silent.

use std::f32::consts::TAU;

#[derive(Clone, Copy, Debug, Default)]
pub struct Ambience {
    pub walking: bool,
    pub running: bool,
    pub forest: f32,
    pub river: f32,
    pub rain: f32,
    pub night: f32,
}

#[derive(Clone, Copy, Debug)]
pub enum Cue {
    Sword,
    Hit,
    Hurt,
    Dodge,
    Loot,
    Quest,
    Forge,
    Talk,
    Defeat,
}

pub struct Audio {
    muted: bool,
    backend: Option<Backend>,
}

impl Audio {
    pub fn new(enabled: bool) -> Self {
        Self {
            muted: false,
            backend: if enabled { Backend::new() } else { None },
        }
    }

    pub fn update(&mut self, _dt: f32, ambience: Ambience) {
        if let Some(backend) = &mut self.backend {
            backend.update(ambience);
        }
    }

    pub fn play(&mut self, cue: Cue) {
        if !self.muted
            && let Some(backend) = &mut self.backend
        {
            backend.play(cue);
        }
    }

    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
        if let Some(backend) = &mut self.backend {
            backend.set_muted(muted);
        }
    }

    pub fn is_muted(&self) -> bool {
        self.muted
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
struct Backend {
    // The playback device must be dropped before its SDL contexts.
    device: sdl2::audio::AudioDevice<Mixer>,
    _audio: sdl2::AudioSubsystem,
    _sdl: sdl2::Sdl,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Backend {
    fn new() -> Option<Self> {
        let sdl = sdl2::init().ok()?;
        let audio = sdl.audio().ok()?;
        let desired = sdl2::audio::AudioSpecDesired {
            freq: Some(44_100),
            channels: Some(2),
            samples: Some(1024),
        };
        let device = audio
            .open_playback(None, &desired, |spec| Mixer::new(spec.freq as f32))
            .ok()?;
        device.resume();
        Some(Self {
            device,
            _audio: audio,
            _sdl: sdl,
        })
    }

    fn update(&mut self, ambience: Ambience) {
        self.device.lock().target = ambience;
    }

    fn play(&mut self, cue: Cue) {
        self.device.lock().cue(cue);
    }

    fn set_muted(&mut self, muted: bool) {
        self.device.lock().muted = muted;
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl sdl2::audio::AudioCallback for Mixer {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        for frame in out.as_chunks_mut::<2>().0 {
            *frame = self.sample();
        }
    }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
struct Backend {
    context: web_sys::AudioContext,
    gain: web_sys::GainNode,
    mixer: Mixer,
    next: f64,
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
impl Backend {
    fn new() -> Option<Self> {
        let context = web_sys::AudioContext::new().ok()?;
        let gain = context.create_gain().ok()?;
        gain.connect_with_audio_node(&context.destination()).ok()?;
        let sample_rate = context.sample_rate();
        Some(Self {
            context,
            gain,
            mixer: Mixer::new(sample_rate),
            next: 0.,
        })
    }

    fn update(&mut self, ambience: Ambience) {
        let started_walking = ambience.walking && !self.mixer.target.walking;
        self.mixer.target = ambience;
        if self.context.state() != web_sys::AudioContextState::Running {
            // Moving is also a first interaction: the player should hear
            // footsteps without having to swing, talk, or toggle sound first.
            // Only try on movement onset, avoiding pending promises each frame
            // in browsers that retain stricter activation restrictions.
            if started_walking {
                let _ = self.context.resume();
            }
            return;
        }
        let now = self.context.current_time();
        self.next = self.next.max(now + 0.005);
        // Short buffers keep footsteps and actions responsive; the browser
        // owns scheduled nodes until they finish playing.
        while self.next < now + 0.08 {
            if self.schedule().is_none() {
                break;
            }
        }
    }

    fn schedule(&mut self) -> Option<()> {
        const FRAMES: usize = 2048;
        let buffer = self
            .context
            .create_buffer(2, FRAMES as u32, self.mixer.rate)
            .ok()?;
        let mut left = vec![0.; FRAMES];
        let mut right = vec![0.; FRAMES];
        for (left, right) in left.iter_mut().zip(&mut right) {
            [*left, *right] = self.mixer.sample();
        }
        buffer.copy_to_channel(&left, 0).ok()?;
        buffer.copy_to_channel(&right, 1).ok()?;
        let source = self.context.create_buffer_source().ok()?;
        source.set_buffer(Some(&buffer));
        source.connect_with_audio_node(&self.gain).ok()?;
        source.start_with_when(self.next).ok()?;
        self.next += FRAMES as f64 / self.mixer.rate as f64;
        Some(())
    }

    fn play(&mut self, cue: Cue) {
        // Browsers allow resume here when called from the player's input.
        let _ = self.context.resume();
        self.mixer.cue(cue);
    }

    fn set_muted(&mut self, muted: bool) {
        self.gain.gain().set_value(if muted { 0. } else { 1. });
        if !muted {
            let _ = self.context.resume();
        }
    }
}

#[cfg(not(any(
    all(not(target_arch = "wasm32"), feature = "desktop"),
    all(target_arch = "wasm32", feature = "web")
)))]
struct Backend;

#[cfg(not(any(
    all(not(target_arch = "wasm32"), feature = "desktop"),
    all(target_arch = "wasm32", feature = "web")
)))]
impl Backend {
    fn new() -> Option<Self> {
        None
    }
    fn update(&mut self, _: Ambience) {}
    fn play(&mut self, _: Cue) {}
    fn set_muted(&mut self, _: bool) {}
}

#[derive(Clone, Copy)]
enum Sound {
    Cue(Cue),
    Step,
    Bird,
}

struct Voice {
    sound: Sound,
    age: f32,
    duration: f32,
    phase: f32,
    pan: f32,
    gain: f32,
}

impl Voice {
    fn sample(&mut self, dt: f32, noise: f32) -> f32 {
        let t = self.age / self.duration;
        self.age += dt;
        let attack = (self.age / 0.008).min(1.);
        let tail = (1. - t).max(0.);
        let (frequency, tone, hiss, decay) = match self.sound {
            Sound::Cue(Cue::Sword) => (380. + 800. * t, 0.025, 0.15, tail.powi(2)),
            Sound::Cue(Cue::Hit) => (135. - 65. * t, 0.23, 0.055, tail.powi(3)),
            Sound::Cue(Cue::Hurt) => (180. - 100. * t, 0.18, 0.02, tail.powi(2)),
            Sound::Cue(Cue::Dodge) => (250., 0.0, 0.09, (std::f32::consts::PI * t).sin().max(0.)),
            Sound::Cue(Cue::Loot) => (
                [784., 988., 1175.][((t * 3.) as usize).min(2)],
                0.10,
                0.,
                tail,
            ),
            Sound::Cue(Cue::Quest) => (
                [523.25, 659.25, 783.99, 1046.5][((t * 4.) as usize).min(3)],
                0.11,
                0.,
                tail.sqrt(),
            ),
            Sound::Cue(Cue::Forge) => (587.33, 0.12, 0.025 * tail.powi(12), tail.powi(3)),
            Sound::Cue(Cue::Talk) => (440. + 90. * t, 0.035, 0., tail.powi(2)),
            Sound::Cue(Cue::Defeat) => (
                [392., 329.63, 261.63][((t * 3.) as usize).min(2)],
                0.085,
                0.,
                tail,
            ),
            Sound::Step => (72., 0.035, 0.07, tail.powi(3)),
            Sound::Bird => (
                2050. + (t * TAU * 2.).sin() * 460. + t * 480.,
                0.045,
                0.,
                (t * std::f32::consts::PI).sin().max(0.).powi(2),
            ),
        };
        self.phase = (self.phase + frequency * dt).fract();
        let pure = (self.phase * TAU).sin();
        let harmonic = if matches!(self.sound, Sound::Cue(Cue::Forge)) {
            (self.phase * TAU * 2.67).sin() * 0.5 + (self.phase * TAU * 4.11).sin() * 0.2
        } else {
            (self.phase * TAU * 2.).sin() * 0.1
        };
        (tone * (pure + harmonic) + hiss * noise) * decay * attack * self.gain
    }
}

struct Mixer {
    rate: f32,
    target: Ambience,
    current: Ambience,
    voices: Vec<Voice>,
    rng: u32,
    low: [f32; 2],
    river_low: [f32; 2],
    step: f32,
    step_side: f32,
    bird: f32,
    time: f32,
    muted: bool,
    volume: f32,
}

impl Mixer {
    fn new(rate: f32) -> Self {
        Self {
            rate,
            target: Ambience::default(),
            current: Ambience::default(),
            voices: Vec::with_capacity(24),
            rng: 0x71b5_43a9,
            low: [0.; 2],
            river_low: [0.; 2],
            step: 0.,
            step_side: 1.,
            bird: 2.,
            time: 0.,
            muted: false,
            volume: 0.,
        }
    }

    fn random(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng as f32 / u32::MAX as f32
    }

    fn add(&mut self, sound: Sound, duration: f32, pan: f32, gain: f32) {
        if self.voices.len() >= 24 {
            self.voices.remove(0);
        }
        self.voices.push(Voice {
            sound,
            age: 0.,
            duration,
            phase: 0.,
            pan,
            gain,
        });
    }

    fn cue(&mut self, cue: Cue) {
        let duration = match cue {
            Cue::Sword => 0.18,
            Cue::Hit => 0.16,
            Cue::Hurt => 0.25,
            Cue::Dodge => 0.22,
            Cue::Loot => 0.33,
            Cue::Quest => 0.8,
            Cue::Forge => 0.75,
            Cue::Talk => 0.12,
            Cue::Defeat => 1.2,
        };
        self.add(Sound::Cue(cue), duration, 0., 1.);
    }

    fn sample(&mut self) -> [f32; 2] {
        let dt = 1. / self.rate;
        self.time = (self.time + dt) % 120.;
        let follow = dt * 2.;
        for (current, target) in [
            (&mut self.current.forest, self.target.forest),
            (&mut self.current.river, self.target.river),
            (&mut self.current.rain, self.target.rain),
            (&mut self.current.night, self.target.night),
        ] {
            let target = if target.is_finite() {
                target.clamp(0., 1.)
            } else {
                0.
            };
            *current += (target - *current) * follow;
        }
        if self.target.walking {
            self.step -= dt;
            if self.step <= 0. {
                self.step = if self.target.running { 0.26 } else { 0.4 };
                self.step_side = -self.step_side;
                self.add(Sound::Step, 0.13, self.step_side * 0.23, 1.);
            }
        } else {
            self.step = 0.;
        }
        self.bird -= dt;
        if self.bird <= 0. {
            self.bird = 2.5 + self.random() * 5.;
            let gain =
                self.current.forest * (1. - self.current.night) * (1. - self.current.rain * 0.8);
            if gain > 0.05 {
                let pan = self.random() * 1.6 - 0.8;
                self.add(Sound::Bird, 0.42, pan, gain);
            }
        }
        let noise = [self.random() * 2. - 1., self.random() * 2. - 1.];
        let breeze = 0.7 + (self.time * 0.23).sin() * 0.3;
        let mut out = [0.; 2];
        for channel in 0..2 {
            self.low[channel] += (noise[channel] - self.low[channel]) * 0.014;
            self.river_low[channel] += (noise[channel] - self.river_low[channel]) * 0.10;
            out[channel] = self.low[channel] * (0.065 * self.current.forest * breeze)
                + self.river_low[channel] * 0.085 * self.current.river
                + (noise[channel] * 0.024 + self.low[channel] * 0.055) * self.current.rain;
        }
        // A quiet pulsing cricket at night; it fades naturally into rain.
        let cricket_gate = ((self.time * 3.7).sin() * 5.).clamp(0., 1.)
            * ((self.time * 72.).sin() * 2.).clamp(0., 1.);
        let cricket = (self.time * 2800. * TAU).sin()
            * 0.004
            * cricket_gate
            * self.current.night
            * self.current.forest
            * (1. - self.current.rain);
        out[0] += cricket;
        out[1] += cricket * 0.7;
        for voice in &mut self.voices {
            let sample = voice.sample(dt, noise[0]);
            out[0] += sample * (1. - voice.pan) * 0.5;
            out[1] += sample * (1. + voice.pan) * 0.5;
        }
        self.voices.retain(|voice| voice.age < voice.duration);
        let target_volume = if self.muted { 0. } else { 0.75 };
        self.volume += (target_volume - self.volume) * (dt * 80.).min(1.);
        [
            out[0].clamp(-0.8, 0.8) * self.volume,
            out[1].clamp(-0.8, 0.8) * self.volume,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_cues_are_finite_audible_and_expire() {
        for cue in [
            Cue::Sword,
            Cue::Hit,
            Cue::Hurt,
            Cue::Dodge,
            Cue::Loot,
            Cue::Quest,
            Cue::Forge,
            Cue::Talk,
            Cue::Defeat,
        ] {
            let mut mixer = Mixer::new(44_100.);
            mixer.cue(cue);
            let mut peak = 0_f32;
            for _ in 0..88_200 {
                let sample = mixer.sample();
                assert!(sample.iter().all(|x| x.is_finite() && x.abs() <= 0.8));
                peak = peak.max(sample[0].abs());
            }
            assert!(peak > 0.005, "inaudible {cue:?}");
            assert!(mixer.voices.is_empty());
        }
    }

    #[test]
    fn ambience_and_muting_stay_bounded() {
        let mut mixer = Mixer::new(44_100.);
        mixer.target = Ambience {
            walking: true,
            running: true,
            forest: 1.,
            river: 1.,
            rain: 1.,
            night: 1.,
        };
        for _ in 0..132_300 {
            assert!(
                mixer
                    .sample()
                    .iter()
                    .all(|x| x.is_finite() && x.abs() <= 0.8)
            );
        }
        mixer.muted = true;
        for _ in 0..22_050 {
            mixer.sample();
        }
        assert!(mixer.sample().iter().all(|x| x.abs() < 0.000001));
        assert!(mixer.voices.len() <= 24);
    }

    #[test]
    fn preview_never_opens_a_device() {
        let mut audio = Audio::new(false);
        audio.play(Cue::Sword);
        audio.set_muted(true);
        audio.update(0.016, Ambience::default());
        assert!(audio.is_muted());
        assert!(audio.backend.is_none());
    }
}
