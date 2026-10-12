//! warp's own sound output (card native-sound, notes/sound.md "Our own API"): the sounds of all voices mixed into one
//! output stream (cpal: CoreAudio on macOS), each starting at its frame, so a sound queued behind another follows it
//! to the sample and `at 2 beats` lands to the sample live too, not only in render_sound. Before, each sound was a WAV
//! for afplay, started by a sleeping thread: exact to the millisecond, with the OS mixing the players.
//! The Mixer is the clockwork, free of any device (tests drive it); the engine feeds it to the device's callback.
//! A sound's start is an Instant (sound.rs scheduled) turned into a frame by one fixed anchor, the stream's start:
//! sounds back to back in Instants stay back to back in frames. LEAD keeps every start ahead of the callback.

use std::time::{Duration, Instant};

/// How far ahead of the device a sound queued for now is scheduled: more than a callback's buffer, so no start is
/// already played when it arrives (CoreAudio buffers hold 512 frames, 11 ms at 44.1 kHz)
const LEAD: Duration = Duration::from_millis(60);
const AMPLITUDE: f32 = 32_768.0;

/// A sound in the mix: its samples at the device's rate and channels, the frame it starts at
struct Mixed {
	handle: usize,
	start: u64,
	samples: Vec<f32>,
}

/// The sounds playing and to play, and the frame the device plays next
pub struct Mixer {
	rate: u32,
	channels: u16,
	frame: u64,
	sounds: Vec<Mixed>,
}

impl Mixer {
	pub fn new(rate: u32, channels: u16) -> Mixer {
		Mixer { rate, channels, frame: 0, sounds: Vec::new() }
	}

	/// The 16-bit samples (`channels` interleaved, at `rate`) starting at the frame, converted to the device's rate
	/// (linear interpolation) and channels (mono on every speaker, stereo's left and right on the first two)
	pub fn add(&mut self, handle: usize, start: u64, samples: &[i16], rate: u32, channels: u16) {
		let channels = channels.max(1) as usize;
		let frames = samples.len() / channels;
		let step = rate as f64 / self.rate as f64;
		let converted_frames = (frames as f64 / step).round() as usize;
		let at = |frame: usize, channel: usize| samples.get(frame * channels + channel.min(channels - 1)).map_or(0.0, |&sample| sample as f32 / AMPLITUDE);
		let mut converted = Vec::with_capacity(converted_frames * self.channels as usize);
		for frame in 0..converted_frames {
			let position = frame as f64 * step;
			let (before, share) = (position.floor() as usize, (position.fract()) as f32);
			for channel in 0..self.channels as usize {
				converted.push(at(before, channel) * (1.0 - share) + at(before + 1, channel) * share);
			}
		}
		self.sounds.push(Mixed { handle, start, samples: converted });
	}

	/// stop_sound(handle), 0 all of them: dropped from the mix at once
	pub fn stop(&mut self, handle: usize) {
		self.sounds.retain(|sound| handle != 0 && sound.handle != handle);
	}

	/// Whether a sound is still to play or playing
	pub fn busy(&self) -> bool {
		!self.sounds.is_empty()
	}

	/// The next frames of the mix into the device's buffer (interleaved, its channels), the finished sounds dropped
	pub fn fill(&mut self, buffer: &mut [f32]) {
		let channels = self.channels as usize;
		buffer.fill(0.0);
		let frames = (buffer.len() / channels) as u64;
		for sound in &self.sounds {
			let sound_frames = (sound.samples.len() / channels) as u64;
			let first = sound.start.max(self.frame);
			let last = (sound.start + sound_frames).min(self.frame + frames);
			for frame in first..last {
				let (into, from) = (((frame - self.frame) as usize) * channels, ((frame - sound.start) as usize) * channels);
				buffer[into..into + channels].iter_mut().zip(&sound.samples[from..from + channels]).for_each(|(out, sample)| *out += sample);
			}
		}
		self.frame += frames;
		let frame = self.frame;
		self.sounds.retain(|sound| sound.start + (sound.samples.len() / channels) as u64 > frame);
		buffer.iter_mut().for_each(|sample| *sample = sample.clamp(-1.0, 1.0));
	}

	/// The frame an instant falls on, the stream started at `anchor`
	pub fn frame_at(&self, instant: Instant, anchor: Instant) -> u64 {
		((instant + LEAD).saturating_duration_since(anchor).as_secs_f64() * self.rate as f64).round() as u64
	}
}

#[cfg(target_os = "macos")]
mod output {
	use super::Mixer;
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
		ENGINE.get_or_init(|| started().map_err(|failure| eprintln!("sound: no output stream ({failure}), playing by afplay")).ok()).as_ref()
	}

	/// The default output device playing the mixer, on a thread that keeps the stream (not Send on every platform)
	fn started() -> Result<Engine, String> {
		let (sender, receiver) = std::sync::mpsc::channel();
		std::thread::spawn(move || {
			let opened = (|| {
				let device = cpal::default_host().default_output_device().ok_or("no output device")?;
				let config: cpal::StreamConfig = device.default_output_config().map_err(|failure| failure.to_string())?.into();
				let mixer = Arc::new(Mutex::new(Mixer::new(config.sample_rate, config.channels)));
				let fed = mixer.clone();
				let stream = device.build_output_stream::<f32, _, _>(config, move |buffer, _| match fed.try_lock() {
					Ok(mut mixer) => mixer.fill(buffer),
					Err(_) => buffer.fill(0.0), // never wait in the audio callback; a queueing sound holds the lock briefly
				}, |failure| eprintln!("sound: {failure}"), None).map_err(|failure| failure.to_string())?;
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
				Err(failure) => { let _ = sender.send(Err(failure)); }
			}
		});
		receiver.recv().map_err(|failure| failure.to_string())?
	}

	/// The sound queued on the device at its start, false without an output stream
	pub fn play(handle: usize, start: Instant, samples: &[i16], rate: u32, channels: u16) -> bool {
		let Some(engine) = engine() else { return false };
		let Ok(mut mixer) = engine.mixer.lock() else { return false };
		let frame = mixer.frame_at(start, engine.anchor);
		mixer.add(handle, frame, samples, rate, channels);
		true
	}

	pub fn stop(handle: usize) {
		if let Some(Ok(mut mixer)) = ENGINE.get().and_then(Option::as_ref).map(|engine| engine.mixer.lock()) {
			mixer.stop(handle);
		}
	}

	pub fn busy() -> bool {
		ENGINE.get().and_then(Option::as_ref).is_some_and(|engine| engine.mixer.lock().is_ok_and(|mixer| mixer.busy()))
	}
}

#[cfg(target_os = "macos")]
pub use output::{busy, play, stop};

/// Without our own output here: the players play every sound
#[cfg(not(target_os = "macos"))]
pub fn play(_: usize, _: Instant, _: &[i16], _: u32, _: u16) -> bool {
	false
}

#[cfg(not(target_os = "macos"))]
pub fn stop(_: usize) {}

#[cfg(not(target_os = "macos"))]
pub fn busy() -> bool {
	false
}
