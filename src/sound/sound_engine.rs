//! warp's own sound output (card native-sound, notes/sound.md "Our own API"): the sounds of all voices mixed into one
//! output stream (cpal: CoreAudio on macOS), each starting at its frame, so a sound queued behind another follows it
//! to the sample and `at 2 beats` lands to the sample live too, not only in save sound. Before, each sound was a WAV
//! for afplay, started by a sleeping thread: exact to the millisecond, with the OS mixing the players.
//! The Mixer is the clockwork, free of any device (tests drive it); the engine feeds it to the device's callback.
//! A sound's start is an Instant (sound.rs scheduled) turned into a frame by one fixed anchor, the stream's start:
//! sounds back to back in Instants stay back to back in frames. LEAD keeps every start ahead of the callback.

use crate::sound::effects::{Chain, Effect};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// How far ahead of the device a sound queued for now is scheduled: more than a callback's buffer, so no start is
/// already played when it arrives (CoreAudio buffers hold 512 frames, 11 ms at 44.1 kHz)
const LEAD: Duration = Duration::from_millis(60);
const AMPLITUDE: f32 = 32_768.0;
/// A sound of the same handle starting this close to where the last one ends continues it (a signal's next block):
/// both starts round to frames on their own
const SEAM_FRAMES: u64 = 2;
/// The most sounds sounding at once (card sound-voice-stealing): one more steals the oldest, which fades out in
/// STEAL_SECONDS and leaves the mix, so live playing never piles up past what the device mixes in time
pub const MAX_VOICES: usize = 32;
const STEAL_SECONDS: f64 = 0.005;

/// A sound in the mix: its samples as given (`channels` interleaved) and their frames a device frame (`ratio`, their
/// rate over the device's), the frame it starts at, its gain and the gain it ramps to by `step` a frame (fade_sound)
struct Mixed {
	handle: usize,
	/// the bus it goes through (card shared-buses), 0: straight to the output
	bus: usize,
	start: u64,
	samples: Vec<f32>,
	channels: usize,
	ratio: f64,
	gain: f32,
	target: f32,
	step: f32,
	stolen: bool,
}

impl Mixed {
	/// Its length in device frames
	fn frames(&self) -> u64 {
		((self.samples.len() / self.channels) as f64 / self.ratio).round() as u64
	}

	/// The device frame's sample on the channel, between the given samples (linear interpolation); mono on every
	/// speaker, stereo's left and right on the first two
	fn sample(&self, frame: u64, channel: usize) -> f32 {
		let position = frame as f64 * self.ratio;
		let (before, share) = (position.floor() as usize, position.fract() as f32);
		let at = |frame: usize| {
			self.samples.get(frame * self.channels + channel.min(self.channels - 1)).copied().unwrap_or(0.0)
		};
		at(before) * (1.0 - share) + at(before + 1) * share
	}

	/// The gain of the frame played now, then one step on toward the target
	fn next_gain(&mut self) -> f32 {
		let gain = self.gain;
		self.gain = if (self.target - gain).abs() <= self.step.abs() { self.target } else { gain + self.step };
		gain
	}
}

/// The sounds playing and to play, the buses they go through and the frame the device plays next
pub struct Mixer {
	rate: u32,
	channels: u16,
	frame: u64,
	sounds: Vec<Mixed>,
	buses: HashMap<usize, Bus>,
}

/// A bus's effect chain, the mix of its sounds in the buffer being filled, whether a sound fed it there, and its
/// effects' changes still to come at their frames (in order)
struct Bus {
	chain: Chain,
	mix: Vec<f32>,
	fed: bool,
	changes: Vec<(u64, Vec<Effect>)>,
}

impl Bus {
	/// The mix of the frames from `from` on through the chain, its effects changing at their frames on the way
	fn process(&mut self, from: u64, channels: usize) {
		let frames = (self.mix.len() / channels) as u64;
		let due = self.changes.iter().take_while(|(frame, _)| *frame < from + frames).count();
		let due: Vec<_> = self.changes.drain(..due).collect();
		if self.fed || !self.chain.rung_out() {
			let at = |frame: u64| frame.saturating_sub(from) as usize * channels;
			self.chain.process_changing(&mut self.mix, due.iter().map(|(frame, effects)| (at(*frame), effects.as_slice())));
		} else {
			due.iter().for_each(|(_, effects)| self.chain.retune(effects));
		}
	}
}

impl Mixer {
	pub fn new(rate: u32, channels: u16) -> Mixer {
		Mixer { rate, channels, frame: 0, sounds: Vec::new(), buses: HashMap::new() }
	}

	/// The 16-bit samples (`channels` interleaved, at `rate`) starting at the frame, played at the device's rate and
	/// channels; continuing the handle's sound that ends there (SEAM_FRAMES), they join it without a seam
	pub fn add(&mut self, handle: usize, start: u64, samples: &[i16], rate: u32, channels: u16) {
		self.add_to_bus(handle, 0, start, samples, rate, channels)
	}

	/// The bus's effects from now on (card shared-buses); its sounds go through them together. A bus set again keeps
	/// what still rings in its effects
	pub fn set_bus(&mut self, bus: usize, effects: Vec<Effect>) {
		let frame = self.frame;
		self.change_bus(bus, frame, effects)
	}

	/// The bus's effects from the frame on (card shared-buses stage 3), a new bus's at once
	pub fn change_bus(&mut self, bus: usize, frame: u64, effects: Vec<Effect>) {
		match self.buses.get_mut(&bus) {
			Some(known) => {
				let at = known.changes.partition_point(|(earlier, _)| *earlier <= frame);
				known.changes.insert(at, (frame, effects));
			}
			None => {
				let chain = Chain::new(&effects, self.rate, self.channels);
				self.buses.insert(bus, Bus { chain, mix: Vec::new(), fed: false, changes: Vec::new() });
			}
		}
	}

	pub fn has_bus(&self, bus: usize) -> bool {
		bus == 0 || self.buses.contains_key(&bus)
	}

	/// add's sound through the bus (0: none, and none for a bus never set)
	pub fn add_to_bus(&mut self, handle: usize, bus: usize, start: u64, samples: &[i16], rate: u32, channels: u16) {
		let (channels, ratio) = (channels.max(1) as usize, rate as f64 / self.rate as f64);
		let samples = samples.iter().map(|&sample| sample as f32 / AMPLITUDE);
		let continued = self.sounds.iter_mut().find(|sound| {
			sound.handle == handle
				&& sound.bus == bus
				&& sound.channels == channels
				&& sound.ratio == ratio
				&& (sound.start + sound.frames()).abs_diff(start) <= SEAM_FRAMES
		});
		match continued {
			Some(sound) => sound.samples.extend(samples),
			None => {
				self.steal_for(start);
				self.sounds.push(Mixed {
					handle,
					bus,
					start,
					samples: samples.collect(),
					channels,
					ratio,
					gain: 1.0,
					target: 1.0,
					step: 0.0,
					stolen: false,
				});
			}
		}
	}

	/// Room for one more sound sounding at the frame: the oldest of MAX_VOICES sounding then fades out and leaves
	fn steal_for(&mut self, start: u64) {
		let rate = self.rate;
		let mut sounding: Vec<&mut Mixed> = self
			.sounds
			.iter_mut()
			.filter(|sound| !sound.stolen && sound.start <= start && start < sound.start + sound.frames())
			.collect();
		if sounding.len() >= MAX_VOICES {
			sounding.sort_by_key(|sound| sound.start);
			let oldest = &mut sounding[0];
			oldest.stolen = true;
			oldest.target = 0.0;
			oldest.step = -oldest.gain / (STEAL_SECONDS * rate as f64).round().max(1.0) as f32;
		}
	}

	/// stop_sound(handle), 0 all of them: dropped from the mix at once
	pub fn stop(&mut self, handle: usize) {
		self.sounds.retain(|sound| handle != 0 && sound.handle != handle);
	}

	/// fade_sound(handle, gain, seconds): the sound's gain ramps linearly to `gain` over the seconds it plays from now on,
	/// sample by sample, so no click; 0 fades all of them
	pub fn fade(&mut self, handle: usize, gain: f32, seconds: f64) {
		let frames = (seconds * self.rate as f64).round().max(1.0) as f32;
		for sound in self.sounds.iter_mut().filter(|sound| handle == 0 || sound.handle == handle) {
			sound.target = gain;
			sound.step = (gain - sound.gain) / frames;
		}
	}

	/// Whether a sound is still to play or playing, or a bus still rings
	pub fn busy(&self) -> bool {
		!self.sounds.is_empty() || self.buses.values().any(|bus| !bus.chain.rung_out())
	}

	/// The next frames of the mix into the device's buffer (interleaved, its channels), the finished sounds dropped
	pub fn fill(&mut self, buffer: &mut [f32]) {
		let channels = self.channels as usize;
		buffer.fill(0.0);
		let frames = (buffer.len() / channels) as u64;
		for bus in self.buses.values_mut() {
			bus.mix.clear();
			bus.mix.resize(buffer.len(), 0.0);
			bus.fed = false;
		}
		for sound in &mut self.sounds {
			let into = match self.buses.get_mut(&sound.bus) {
				Some(bus) => {
					bus.fed = true;
					&mut bus.mix
				}
				None => &mut *buffer,
			};
			mixed_into(sound, into, self.frame, channels);
		}
		for bus in self.buses.values_mut() {
			bus.process(self.frame, channels);
			buffer.iter_mut().zip(&bus.mix).for_each(|(out, sample)| *out += sample);
		}
		self.frame += frames;
		let frame = self.frame;
		self.sounds.retain(|sound| sound.start + sound.frames() > frame && !(sound.stolen && sound.gain == 0.0));
		buffer.iter_mut().for_each(|sample| *sample = sample.clamp(-1.0, 1.0));
	}

	/// The frame an instant falls on, the stream started at `anchor`
	pub fn frame_at(&self, instant: Instant, anchor: Instant) -> u64 {
		((instant + LEAD).saturating_duration_since(anchor).as_secs_f64() * self.rate as f64).round() as u64
	}
}

/// The sound's frames that fall into the buffer starting at the frame, added to it at its gain
fn mixed_into(sound: &mut Mixed, buffer: &mut [f32], from: u64, channels: usize) {
	let frames = (buffer.len() / channels) as u64;
	let first = sound.start.max(from);
	let last = (sound.start + sound.frames()).min(from + frames);
	for frame in first..last {
		let into = ((frame - from) as usize) * channels;
		let gain = sound.next_gain();
		for (channel, out) in buffer[into..into + channels].iter_mut().enumerate() {
			*out += sound.sample(frame - sound.start, channel) * gain;
		}
	}
}

#[cfg(target_os = "macos")]
mod output {
	use super::{Effect, Mixer};
	use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
	use std::sync::{Arc, Mutex, OnceLock};
	use std::time::Instant;

	/// The mixer the device plays and the instant its frame 0 stands for
	struct Engine {
		mixer: Arc<Mutex<Mixer>>,
		anchor: Instant,
	}

	/// The engine, started by the first sound; None when no output device opens (the players take over)
	static ENGINE: OnceLock<Option<Engine>> = OnceLock::new();

	fn engine() -> Option<&'static Engine> {
		ENGINE
			.get_or_init(|| {
				started().map_err(|failure| eprintln!("sound: no output stream ({failure}), playing by afplay")).ok()
			})
			.as_ref()
	}

	/// The default output device playing the mixer, on a thread that keeps the stream (not Send on every platform)
	fn started() -> Result<Engine, String> {
		let (sender, receiver) = std::sync::mpsc::channel();
		std::thread::spawn(move || {
			let opened = (|| {
				let device = cpal::default_host().default_output_device().ok_or("no output device")?;
				let config: cpal::StreamConfig =
					device.default_output_config().map_err(|failure| failure.to_string())?.into();
				let mixer = Arc::new(Mutex::new(Mixer::new(config.sample_rate, config.channels)));
				let fed = mixer.clone();
				let stream = device
					.build_output_stream::<f32, _, _>(
						config,
						move |buffer, _| match fed.try_lock() {
							Ok(mut mixer) => mixer.fill(buffer),
							Err(_) => buffer.fill(0.0), // never wait in the audio callback; a queueing sound holds the lock briefly
						},
						|failure| eprintln!("sound: {failure}"),
						None,
					)
					.map_err(|failure| failure.to_string())?;
				stream.play().map_err(|failure| failure.to_string())?;
				Ok::<_, String>((stream, Engine { mixer, anchor: Instant::now() }))
			})();
			match opened {
				Ok((stream, engine)) => {
					let _ = sender.send(Ok(engine));
					let _stream = stream;
					loop {
						std::thread::park();
					}
				}
				Err(failure) => {
					let _ = sender.send(Err(failure));
				}
			}
		});
		receiver.recv().map_err(|failure| failure.to_string())?
	}

	/// Whether the device plays warp's own output stream (started now, if not yet)
	pub fn available() -> bool {
		engine().is_some()
	}

	/// The sound queued on the device at its start through the bus (0: none), whose effects the mixer learns with its
	/// first sound; false without an output stream
	pub fn play(
		handle: usize,
		bus: (usize, &[Effect]),
		start: Instant,
		samples: &[i16],
		rate: u32,
		channels: u16,
	) -> bool {
		let Some(engine) = engine() else { return false };
		let Ok(mut mixer) = engine.mixer.lock() else {
			return false;
		};
		let frame = mixer.frame_at(start, engine.anchor);
		if !mixer.has_bus(bus.0) {
			mixer.set_bus(bus.0, bus.1.to_vec());
		}
		mixer.add_to_bus(handle, bus.0, frame, samples, rate, channels);
		true
	}

	/// The bus's effects changed, from now on
	pub fn set_bus(bus: usize, effects: &[Effect]) {
		with_mixer(|mixer| mixer.set_bus(bus, effects.to_vec()));
	}

	/// The bus's effects changed from the instant on, which a voice ahead of the device names
	pub fn change_bus(bus: usize, start: Instant, effects: &[Effect]) {
		let Some(engine) = ENGINE.get().and_then(Option::as_ref) else { return };
		with_mixer(|mixer| {
			let frame = mixer.frame_at(start, engine.anchor);
			mixer.change_bus(bus, frame, effects.to_vec())
		});
	}

	/// The running engine's mixer changed or read; None before the first sound or without an output stream
	fn with_mixer<T>(action: impl FnOnce(&mut Mixer) -> T) -> Option<T> {
		let engine = ENGINE.get()?.as_ref()?;
		engine.mixer.lock().ok().map(|mut mixer| action(&mut mixer))
	}

	pub fn stop(handle: usize) {
		with_mixer(|mixer| mixer.stop(handle));
	}

	pub fn busy() -> bool {
		with_mixer(|mixer| mixer.busy()).unwrap_or(false)
	}

	/// fade_sound on the device, false without our own output (sounds the players play keep their gain)
	pub fn fade(handle: usize, gain: f32, seconds: f64) -> bool {
		with_mixer(|mixer| mixer.fade(handle, gain, seconds)).is_some()
	}
}

pub use output::{available, busy, change_bus, fade, play, set_bus, stop};

/// Without our own output here: the players play every sound
#[cfg(not(target_os = "macos"))]
mod output {
	use super::Effect;
	use std::time::Instant;

	pub fn available() -> bool {
		false
	}

	pub fn play(_: usize, _: (usize, &[Effect]), _: Instant, _: &[i16], _: u32, _: u16) -> bool {
		false
	}

	pub fn set_bus(_: usize, _: &[Effect]) {}

	pub fn change_bus(_: usize, _: Instant, _: &[Effect]) {}

	pub fn stop(_: usize) {}

	pub fn busy() -> bool {
		false
	}

	pub fn fade(_: usize, _: f32, _: f64) -> bool {
		false
	}
}
