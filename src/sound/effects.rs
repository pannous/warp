//! A bus's effect chain (card shared-buses): the sounds sent to a bus mixed, then processed together as a stream, frame
//! by frame, so a reverb or delay sounds once for all of them and rings on after the last. The same formulas as
//! lib/sound.warp's effects on a finished signal (biquad, echoed, roomed, clipped), amplitudes full scale -1…1.

/// The delays (seconds) and the feedback of the reverb's four parallel combs (Schroeder), as lib/sound.warp's
const REVERB_DELAYS: [f32; 4] = [0.0297, 0.0371, 0.0411, 0.0437];
const REVERB_FEEDBACK: f32 = 0.75;
/// A bus whose output stays below this (-60 dB) for TAIL_SECONDS has rung out
const SILENT: f32 = 1e-3;
const TAIL_SECONDS: f32 = 0.05;

/// One effect of a chain and its settings
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Effect {
	Gain(f32),
	/// lowpass, highpass or bandpass at the frequency (Hz) with the resonance q
	Filter {
		kind: FilterKind,
		frequency: f32,
		q: f32,
	},
	/// echoes every `seconds`, each at `feedback` of the one before, `mix` of them against the dry sound
	Delay {
		seconds: f32,
		feedback: f32,
		mix: f32,
	},
	/// a small room's tail, `mix` of it against the dry sound
	Reverb {
		mix: f32,
	},
	/// a soft clipper's drive (0: clean)
	Distort {
		drive: f32,
	},
	/// a level over `threshold` (dB full scale) comes out `ratio` times closer to it, its envelope following a rise in
	/// `attack` seconds and a fall in `release` (stage 4)
	Compress {
		threshold: f32,
		ratio: f32,
		attack: f32,
		release: f32,
	},
}

/// A biquad's response; the EQ's peak and shelves (stage 4) raise or lower by their dB
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FilterKind {
	Lowpass,
	Highpass,
	Bandpass,
	Peak(f32),
	LowShelf(f32),
	HighShelf(f32),
}

impl Effect {
	/// `["lowpass", 800, 0.7]`, `["delay", 0.25, 0.4, 0.5]`, `["reverb", 0.3]`, `["distort", 0.5]`, `["gain", 0.5]`: an
	/// effect as lib/sound.warp names it, its settings in order
	pub fn named(name: &str, settings: &[f64]) -> Result<Effect, String> {
		let setting = |index: usize| {
			settings
				.get(index)
				.map(|&value| value as f32)
				.ok_or_else(|| format!("the effect {name} needs {} settings", index + 1))
		};
		let filter = |kind| Ok(Effect::Filter { kind, frequency: setting(0)?, q: setting(1)? });
		// `["peak", 1000, 6, 1]`: frequency, dB, q
		let equalizer = |kind: fn(f32) -> FilterKind| {
			Ok(Effect::Filter { kind: kind(setting(1)?), frequency: setting(0)?, q: setting(2)? })
		};
		match name {
			"gain" => Ok(Effect::Gain(setting(0)?)),
			"lowpass" => filter(FilterKind::Lowpass),
			"highpass" => filter(FilterKind::Highpass),
			"bandpass" => filter(FilterKind::Bandpass),
			"peak" => equalizer(FilterKind::Peak),
			"low_shelf" => equalizer(FilterKind::LowShelf),
			"high_shelf" => equalizer(FilterKind::HighShelf),
			"delay" => Ok(Effect::Delay { seconds: setting(0)?, feedback: setting(1)?, mix: setting(2)? }),
			"reverb" => Ok(Effect::Reverb { mix: setting(0)? }),
			"distort" => Ok(Effect::Distort { drive: setting(0)? }),
			"compress" => Ok(Effect::Compress {
				threshold: setting(0)?,
				ratio: setting(1)?,
				attack: setting(2)?,
				release: setting(3)?,
			}),
			_ => Err(format!(
				"no effect named {name}: gain, lowpass, highpass, bandpass, peak, low_shelf, high_shelf, delay, reverb, distort or compress"
			)),
		}
	}
}

/// A chain as JSON, `[["reverb", 0.4], ["delay", 0.25, 0.4, 0.5]]`: how the playground hands a bus's chain over
/// (host-sound.js, card shared-buses stage 2)
pub fn chain_of_json(json: &str) -> Result<Vec<Effect>, String> {
	let effects: Vec<Vec<serde_json::Value>> = serde_json::from_str(json).map_err(|error| error.to_string())?;
	effects
		.iter()
		.map(|effect| {
			let name = effect.first().and_then(serde_json::Value::as_str).unwrap_or_default();
			let settings: Vec<f64> = effect[1..].iter().filter_map(serde_json::Value::as_f64).collect();
			Effect::named(name, &settings)
		})
		.collect()
}

/// The past samples a feedback comb reads back, per channel: y[i] = x[i] + feedback · y[i - gap]
struct Comb {
	echoes: Vec<f32>,
	at: usize,
	feedback: f32,
}

impl Comb {
	fn new(seconds: f32, feedback: f32, rate: u32) -> Comb {
		let gap = ((seconds * rate as f32).round() as usize).max(1);
		Comb { echoes: vec![0.0; gap], at: 0, feedback }
	}

	/// new settings, its echoes kept (a longer gap reads silence where it grew)
	fn retune(&mut self, seconds: f32, feedback: f32, rate: u32) {
		let gap = ((seconds * rate as f32).round() as usize).max(1);
		self.echoes.resize(gap, 0.0);
		self.at %= gap;
		self.feedback = feedback;
	}

	fn next(&mut self, input: f32) -> f32 {
		let output = input + self.feedback * self.echoes[self.at];
		self.echoes[self.at] = output;
		self.at = (self.at + 1) % self.echoes.len();
		output
	}
}

/// The biquad's coefficients (lib/sound.warp biquad, the RBJ cookbook) and its last two inputs and outputs
struct Biquad {
	b: [f32; 3],
	a: [f32; 2],
	inputs: [f32; 2],
	outputs: [f32; 2],
}

impl Biquad {
	fn new(kind: FilterKind, frequency: f32, q: f32, rate: u32) -> Biquad {
		let w = std::f32::consts::TAU * frequency / rate as f32;
		let (alpha, c) = (w.sin() / (2.0 * q.max(1e-3)), w.cos());
		// the cookbook's b0 b1 b2 a0 a1 a2; A the amplitude of the EQ's dB, its square root for the shelves' slope
		let amplitude = |db: f32| 10f32.powf(db / 40.0);
		let shelf = |a: f32, sign: f32| {
			let (plus, minus, slope) = (a + 1.0, a - 1.0, 2.0 * a.sqrt() * alpha);
			[
				a * (plus - sign * minus * c + slope),
				sign * 2.0 * a * (minus - sign * plus * c),
				a * (plus - sign * minus * c - slope),
				plus + sign * minus * c + slope,
				-sign * 2.0 * (minus + sign * plus * c),
				plus + sign * minus * c - slope,
			]
		};
		let [b0, b1, b2, a0, a1, a2] = match kind {
			FilterKind::Lowpass => [(1.0 - c) / 2.0, 1.0 - c, (1.0 - c) / 2.0, 1.0 + alpha, -2.0 * c, 1.0 - alpha],
			FilterKind::Highpass => [(1.0 + c) / 2.0, -(1.0 + c), (1.0 + c) / 2.0, 1.0 + alpha, -2.0 * c, 1.0 - alpha],
			FilterKind::Bandpass => [alpha, 0.0, -alpha, 1.0 + alpha, -2.0 * c, 1.0 - alpha],
			FilterKind::Peak(db) => {
				let a = amplitude(db);
				[1.0 + alpha * a, -2.0 * c, 1.0 - alpha * a, 1.0 + alpha / a, -2.0 * c, 1.0 - alpha / a]
			}
			FilterKind::LowShelf(db) => shelf(amplitude(db), 1.0),
			FilterKind::HighShelf(db) => shelf(amplitude(db), -1.0),
		};
		Biquad { b: [b0 / a0, b1 / a0, b2 / a0], a: [-a1 / a0, -a2 / a0], inputs: [0.0; 2], outputs: [0.0; 2] }
	}

	/// new coefficients, its past samples kept
	fn retune(&mut self, kind: FilterKind, frequency: f32, q: f32, rate: u32) {
		let fresh = Biquad::new(kind, frequency, q, rate);
		(self.b, self.a) = (fresh.b, fresh.a);
	}

	fn next(&mut self, x: f32) -> f32 {
		let y = self.b[0] * x
			+ self.b[1] * self.inputs[0]
			+ self.b[2] * self.inputs[1]
			+ self.a[0] * self.outputs[0]
			+ self.a[1] * self.outputs[1];
		self.inputs = [x, self.inputs[0]];
		self.outputs = [y, self.outputs[0]];
		y
	}
}

/// An effect with its running state, one per channel where it remembers samples
enum Stage {
	Gain(f32),
	Filter(Vec<Biquad>),
	Delay { combs: Vec<Comb>, mix: f32 },
	Reverb { rooms: Vec<[Comb; 4]>, mix: f32 },
	Distort(f32),
	Compress(Compressor),
}

/// A compressor: its envelope of the louder channel (linked, so the stereo image stays), and its settings as factors
/// a frame
struct Compressor {
	envelope: f32,
	threshold: f32,
	slope: f32,
	attack: f32,
	release: f32,
}

impl Compressor {
	fn new(threshold: f32, ratio: f32, attack: f32, release: f32, rate: u32) -> Compressor {
		let mut compressor = Compressor { envelope: 0.0, threshold: 0.0, slope: 0.0, attack: 0.0, release: 0.0 };
		compressor.retune(threshold, ratio, attack, release, rate);
		compressor
	}

	/// new settings, its envelope kept
	fn retune(&mut self, threshold: f32, ratio: f32, attack: f32, release: f32, rate: u32) {
		// the share of the envelope a sample keeps: it closes 1 - 1/e of the way to the level in the seconds
		let kept = |seconds: f32| (-1.0 / (seconds.max(1e-5) * rate as f32)).exp();
		(self.threshold, self.slope) = (threshold, 1.0 - 1.0 / ratio.max(1.0));
		(self.attack, self.release) = (kept(attack), kept(release));
	}

	/// The frame's channels turned down together by the envelope over its loudest
	fn process(&mut self, frame: &mut [f32]) {
		let level = frame.iter().fold(0.0, |loudest: f32, sample| sample.abs().max(loudest));
		let kept = if level > self.envelope { self.attack } else { self.release };
		self.envelope = level + kept * (self.envelope - level);
		let over = 20.0 * self.envelope.max(1e-9).log10() - self.threshold;
		if over > 0.0 {
			let gain = 10f32.powf(-over * self.slope / 20.0);
			frame.iter_mut().for_each(|sample| *sample *= gain);
		}
	}
}

impl Stage {
	fn new(effect: &Effect, rate: u32, channels: usize) -> Stage {
		let each = |make: &dyn Fn() -> Comb| (0..channels).map(|_| make()).collect::<Vec<_>>();
		match *effect {
			Effect::Gain(gain) => Stage::Gain(gain),
			Effect::Filter { kind, frequency, q } => {
				Stage::Filter((0..channels).map(|_| Biquad::new(kind, frequency, q, rate)).collect())
			}
			Effect::Delay { seconds, feedback, mix } => {
				Stage::Delay { combs: each(&|| Comb::new(seconds, feedback, rate)), mix }
			}
			Effect::Reverb { mix } => Stage::Reverb {
				rooms: (0..channels)
					.map(|_| REVERB_DELAYS.map(|delay| Comb::new(delay, REVERB_FEEDBACK, rate)))
					.collect(),
				mix,
			},
			Effect::Distort { drive } => Stage::Distort(drive),
			Effect::Compress { threshold, ratio, attack, release } => {
				Stage::Compress(Compressor::new(threshold, ratio, attack, release, rate))
			}
		}
	}

	/// The effect's new settings when it is of the stage's kind, what rings in it kept; false for another kind
	fn retune(&mut self, effect: &Effect, rate: u32) -> bool {
		match (self, *effect) {
			(Stage::Gain(gain), Effect::Gain(new)) => *gain = new,
			(Stage::Filter(filters), Effect::Filter { kind, frequency, q }) => {
				filters.iter_mut().for_each(|filter| filter.retune(kind, frequency, q, rate))
			}
			(Stage::Delay { combs, mix }, Effect::Delay { seconds, feedback, mix: new }) => {
				combs.iter_mut().for_each(|comb| comb.retune(seconds, feedback, rate));
				*mix = new;
			}
			(Stage::Reverb { mix, .. }, Effect::Reverb { mix: new }) => *mix = new,
			(Stage::Distort(drive), Effect::Distort { drive: new }) => *drive = new,
			(Stage::Compress(compressor), Effect::Compress { threshold, ratio, attack, release }) => {
				compressor.retune(threshold, ratio, attack, release, rate)
			}
			_ => return false,
		}
		true
	}

	/// The frames it reads back: its longest comb's gap
	fn memory(&self) -> usize {
		match self {
			Stage::Delay { combs, .. } => combs.first().map_or(0, |comb| comb.echoes.len()),
			Stage::Reverb { rooms, .. } => {
				rooms.first().map_or(0, |room| room.iter().map(|comb| comb.echoes.len()).max().unwrap_or(0))
			}
			_ => 0,
		}
	}

	/// The frame (its channels) through the stage, in place
	fn process(&mut self, frame: &mut [f32]) {
		match self {
			Stage::Compress(compressor) => compressor.process(frame),
			_ => frame.iter_mut().enumerate().for_each(|(channel, sample)| *sample = self.next(channel, *sample)),
		}
	}

	fn next(&mut self, channel: usize, x: f32) -> f32 {
		match self {
			Stage::Gain(gain) => x * *gain,
			Stage::Filter(filters) => filters[channel].next(x),
			Stage::Delay { combs, mix } => x + *mix * (combs[channel].next(x) - x),
			Stage::Reverb { rooms, mix } => {
				let wet: f32 = rooms[channel].iter_mut().map(|comb| comb.next(x)).sum();
				x + *mix * wet / REVERB_DELAYS.len() as f32
			}
			// lib/sound.warp clipped, its peak full scale here
			Stage::Distort(drive) => (x * (1.0 + 10.0 * *drive)).tanh(),
			Stage::Compress(_) => unreachable!("a compressor runs on whole frames (process)"),
		}
	}
}

/// The effects of a bus in order, running on interleaved frames of its channels
pub struct Chain {
	stages: Vec<Stage>,
	rate: u32,
	channels: usize,
	quiet_frames: usize,
	tail_frames: usize,
}

impl Chain {
	pub fn new(effects: &[Effect], rate: u32, channels: u16) -> Chain {
		let channels = channels.max(1) as usize;
		let mut chain = Chain { stages: Vec::new(), rate, channels, quiet_frames: 0, tail_frames: 0 };
		chain.retune(effects);
		chain.quiet_frames = chain.tail_frames;
		chain
	}

	/// The effects' new settings from the next frame on (card shared-buses stage 3): an effect of the kind its stage
	/// has keeps what rings in it, another kind starts afresh
	pub fn retune(&mut self, effects: &[Effect]) {
		let mut old = std::mem::take(&mut self.stages).into_iter();
		let (rate, channels) = (self.rate, self.channels);
		self.stages = effects
			.iter()
			.map(|effect| {
				old.next()
					.and_then(|mut stage| stage.retune(effect, rate).then_some(stage))
					.unwrap_or_else(|| Stage::new(effect, rate, channels))
			})
			.collect();
		// quiet between two echoes is no end yet: a whole gap of the longest comb has to stay quiet
		let longest_gap = self.stages.iter().map(Stage::memory).max().unwrap_or(0);
		self.tail_frames = longest_gap + (TAIL_SECONDS * rate as f32) as usize;
	}

	/// The frames (interleaved) through every effect, in place
	pub fn process(&mut self, frames: &mut [f32]) {
		for frame in frames.chunks_mut(self.channels) {
			self.stages.iter_mut().for_each(|stage| stage.process(frame));
			let quiet = frame.iter().all(|sample| sample.abs() < SILENT);
			self.quiet_frames = if quiet { self.quiet_frames + 1 } else { 0 };
		}
	}

	/// Whether its echoes and tail have died away: nothing comes out without more input
	pub fn rung_out(&self) -> bool {
		self.quiet_frames >= self.tail_frames
	}
}

/// The samples (interleaved) through the effects, which change at the frames of `changes` (in order), followed by
/// their tail until it has rung out
pub fn processed(
	samples: &[f32],
	effects: &[Effect],
	changes: &[(usize, Vec<Effect>)],
	rate: u32,
	channels: u16,
) -> Vec<f32> {
	let mut chain = Chain::new(effects, rate, channels);
	let mut output = samples.to_vec();
	let mut done = 0;
	for (frame, effects) in changes {
		let until = (frame * channels.max(1) as usize).max(done);
		if output.len() < until {
			output.resize(until, 0.0);
		}
		chain.process(&mut output[done..until]);
		chain.retune(effects);
		done = until;
	}
	chain.process(&mut output[done..]);
	let block = channels.max(1) as usize * rate as usize / 100;
	while !chain.rung_out() {
		let mut tail = vec![0.0; block];
		chain.process(&mut tail);
		output.extend(tail);
	}
	output
}
