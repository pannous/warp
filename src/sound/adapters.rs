//! The sound module's adapter words (lib/sound.warp's std_io "sound" and "midi" calls, src/modules/std_adapters.rs
//! answers them through here): plays, buses, renders, MIDI files and ports

use crate::modules::std_adapters::{list_of, no_such_word, number_of, text_of, texts, whole_numbers, write_file};
use crate::node::Node;

/// `play "song.mp3"`, `stop_sound`, `fade_sound` (lib/sound.warp, cards sound-library, sound-pro, native-sound)
pub(crate) fn sound(member: &str, arguments: &[Node]) -> Result<Node, String> {
	match (member, arguments) {
		("play_file", [path]) => {
			return crate::sound::play_file(&beside_program(path)?).map(|handle| Node::int(handle as i64))
		}
		// stop(handle), play's handle; handle 0: all of them
		("stop", [handle]) if number_of(handle)? == 0.0 => crate::sound::stop(),
		("stop", [handle]) => crate::sound::stop_one(whole_of(handle)?)?,
		("fade", [handle, gain, seconds]) => {
			crate::sound::fade(whole_of(handle)?, number_of(gain)?, number_of(seconds)?)?
		}
		("at", [seconds]) => crate::sound::at(number_of(seconds)?),
		// card shared-buses: bus(number, [[name, settings…] …]) its effects (number 0: a new bus), send(bus) the next sounds, returning the bus before
		("bus", [bus, effects]) => {
			let effects = effects.as_items().iter().map(effect_of).collect::<Result<Vec<_>, String>>()?;
			return crate::sound::define_bus(whole_of(bus)?, effects).map(|bus| Node::int(bus as i64));
		}
		// effect([name, settings…], signal, rate, full scale): a list signal through the bus DSP (card shared-buses stage 2)
		("effect", [effect, signal, rate, full_scale]) => {
			let signal = signal.as_items().iter().map(number_of).collect::<Result<Vec<_>, String>>()?;
			let processed = crate::sound::effects::on_signal(
				effect_of(effect)?,
				&signal,
				number_of(rate)?.max(1.0) as u32,
				number_of(full_scale)?,
			);
			return Ok(Node::list(processed.into_iter().map(Node::float).collect()));
		}
		("send", [bus]) => return Ok(Node::int(crate::sound::send_to(whole_of(bus)?) as i64)),
		("at_end", []) => crate::sound::at_end(),
		("rest", [seconds]) => crate::sound::rest(number_of(seconds)?),
		("wait", []) => crate::sound::wait(),
		("last", []) => return Ok(Node::int(crate::sound::last_handle() as i64)),
		// a live loop's cycle starts (lib live_cycle_started): only the playground hands loops over between runs
		("live_cycle", [_]) => {}
		("queued", []) => return Ok(Node::float(crate::sound::queued_seconds())),
		("render", [path]) => return crate::sound::render(&text_of(path)?).map(Node::float),
		("load", [path]) => {
			return crate::sound::audio_files::load(&text_of(path)?)
				.map(|flat| Node::list(flat.into_iter().map(Node::int).collect()))
		}
		// spectrum(samples, seconds, bands): the levels of the bands in the window of samples (src/sound/spectrum.rs)
		("spectrum", [window, rate, bands, low, high]) => {
			let window = window
				.as_items()
				.iter()
				.map(|sample| Ok(number_of(sample)? - crate::sound::SAMPLE_OFFSET as f64))
				.collect::<Result<Vec<_>, String>>()?;
			return band_levels(&window, number_of(rate)?, bands, low, high);
		}
		// spectrum("song.mp3", seconds, bands): the same of the file's window at that moment, the file decoded once
		("file_spectrum", [path, seconds, window_seconds, bands, low, high]) => {
			let mono = crate::sound::audio_files::mono(&beside_program(path)?)?;
			let window = crate::sound::spectrum::window_at(
				&mono.amplitudes,
				mono.rate,
				number_of(seconds)?,
				number_of(window_seconds)?,
			);
			return band_levels(&window, mono.rate, bands, low, high);
		}
		("file_seconds", [path]) => {
			let mono = crate::sound::audio_files::mono(&beside_program(path)?)?;
			return Ok(Node::float(mono.amplitudes.len() as f64 / mono.rate));
		}
		_ => return no_such_word(arguments),
	}
	Ok(Node::Empty)
}

/// The levels of `bands` bands from low to high Hz in the window (src/sound/spectrum.rs), as a list
fn band_levels(window: &[f64], rate: f64, bands: &Node, low: &Node, high: &Node) -> Result<Node, String> {
	let bands = whole_of(bands)?;
	let levels = crate::sound::spectrum::window_levels(window, rate, bands, number_of(low)?, number_of(high)?);
	Ok(Node::list(levels.into_iter().map(Node::float).collect()))
}

/// A handle, bus or count: a number cut to a whole one, 0 at least
fn whole_of(node: &Node) -> Result<usize, String> {
	Ok(number_of(node)?.max(0.0) as usize)
}

/// A file's path, beside the program when it is there
fn beside_program(path: &Node) -> Result<String, String> {
	Ok(crate::modules::beside_program_if_there(&text_of(path)?))
}

/// `["delay", 0.25, 0.4, 0.5]`: an effect by its name and settings (src/sound/effects.rs Effect::named)
fn effect_of(effect: &Node) -> Result<crate::sound::effects::Effect, String> {
	let items = effect.as_items();
	let (name, settings) = items.split_first().ok_or("an effect is [name, settings…]")?;
	let settings = settings.iter().map(number_of).collect::<Result<Vec<_>, String>>()?;
	crate::sound::effects::Effect::named(&text_of(name)?, &settings)
}

/// MIDI (card midi-out, src/sound/midi.rs): notes cross as flat whole numbers [start ms, key, length ms, velocity …]
pub(crate) fn midi(member: &str, arguments: &[Node]) -> Result<Node, String> {
	match (member, arguments) {
		("write_file", [path, notes]) => {
			write_file(&text_of(path)?, crate::sound::midi::encode(&whole_numbers(notes)?)?)
		}
		("read_file", [path]) => {
			let path = text_of(path)?;
			let bytes = std::fs::read(&path).map_err(|problem| format!("cannot read {path}: {problem}"))?;
			crate::sound::midi::decode(&bytes).map(|notes| Node::list(notes.into_iter().map(Node::int).collect()))
		}
		("send", [port, message]) => {
			let bytes = whole_numbers(message)?
				.into_iter()
				.map(|byte| u8::try_from(byte).map_err(|_| format!("{byte} is no MIDI byte (0 to 255)")))
				.collect::<Result<Vec<_>, _>>()?;
			crate::sound::midi::send(&text_of(port)?, &bytes).map(|_| Node::Empty)
		}
		("outputs", []) => crate::sound::midi::outputs().map(texts),
		("inputs", []) => crate::sound::midi::inputs().map(texts),
		("receive", [port, seconds]) => {
			crate::sound::midi::receive(&text_of(port)?, number_of(seconds)?).map(|message| {
				message.map_or(Node::Empty, |bytes| list_of(bytes.into_iter().map(|byte| Node::int(byte as i64))))
			})
		}
		_ => no_such_word(arguments),
	}
}
