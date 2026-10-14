//! MIDI (card midi-out, notes/sound.md layer 7): Standard MIDI Files and live output.
//!
//! A note crosses to the host as four whole numbers, start in milliseconds, key (0..127), length in milliseconds and
//! velocity (1..127); a file is a list of them flattened. Files are format 0, 480 ticks a quarter at 120 beats per
//! minute (960 ticks a second), so a millisecond is a whole number of ticks to within 5%: ticks are rounded, not kept.
//! Live output goes to the system's MIDI devices through midir (CoreMIDI, ALSA, WinMM) when the user may hear it;
//! headless (tests, CI, WARP_NO_WINDOW) only a `midi out <port>: 90 3c 64` line on stderr, as sound prints its WAV. A
//! message goes out at its voice's moment like a sound (card midi-clock): after the sounds before it, or at an `at`'s
//! time. Live input (card midi-receive) comes natively from an input kept open for the run; the playground's Web MIDI
//! input stays on the page, out of the program's Worker, and fails loudly there.

use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use std::collections::HashMap;
use std::sync::{mpsc, LazyLock, Mutex};

const TICKS_PER_QUARTER: u32 = 480;
const MICROSECONDS_PER_QUARTER: u32 = 500_000;
const TICKS_PER_SECOND: f64 = TICKS_PER_QUARTER as f64 * 1_000_000.0 / MICROSECONDS_PER_QUARTER as f64;
const NOTE_ON: u8 = 0x90;
const NOTE_OFF: u8 = 0x80;
/// The name the system shows for warp's MIDI client and its connections
const CLIENT: &str = "warp";
/// A note's four numbers in the flat list
pub const FIELDS: usize = 4;

/// The outputs opened by name, kept open for the run
static OUTPUTS: LazyLock<Mutex<HashMap<String, MidiOutputConnection>>> = LazyLock::new(Default::default);
/// The inputs opened by name, their messages queued
static INPUTS: LazyLock<Mutex<HashMap<String, Input>>> = LazyLock::new(Default::default);

fn variable_length(mut value: u32, bytes: &mut Vec<u8>) {
	let mut groups = vec![(value & 0x7f) as u8];
	value >>= 7;
	while value > 0 {
		groups.push((value & 0x7f) as u8 | 0x80);
		value >>= 7;
	}
	bytes.extend(groups.iter().rev());
}

fn ticks(milliseconds: i64) -> u32 {
	(milliseconds.max(0) as f64 * TICKS_PER_SECOND / 1000.0).round() as u32
}

fn chunk(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
	[kind.as_slice(), &(body.len() as u32).to_be_bytes(), body].concat()
}

/// The bytes of a format 0 file holding the notes `[start ms, key, length ms, velocity …]`
pub fn encode(flat: &[i64]) -> Result<Vec<u8>, String> {
	if !flat.len().is_multiple_of(FIELDS) {
		return Err(format!(
			"notes come as [start, key, length, velocity] four numbers each, got {} numbers",
			flat.len()
		));
	}
	let mut events = vec![];
	for &[start, key, length, velocity] in flat.as_chunks::<FIELDS>().0 {
		if !(0..=127).contains(&key) {
			return Err(format!("MIDI keys are 0 to 127, got {key}"));
		}
		let velocity = velocity.clamp(1, 127) as u8;
		let (on, off) = (ticks(start), ticks(start + length));
		events.push((on, 1, [NOTE_ON, key as u8, velocity]));
		events.push((off.max(on), 0, [NOTE_OFF, key as u8, 0])); // at one tick an off sorts before the next on
	}
	events.sort_by_key(|(tick, order, _)| (*tick, *order));
	let mut track = vec![0, 0xff, 0x51, 3];
	track.extend(&MICROSECONDS_PER_QUARTER.to_be_bytes()[1..]);
	let mut now = 0;
	for (tick, _, message) in events {
		variable_length(tick - now, &mut track);
		track.extend(message);
		now = tick;
	}
	track.extend([0, 0xff, 0x2f, 0]);
	let header = [0u16.to_be_bytes(), 1u16.to_be_bytes(), (TICKS_PER_QUARTER as u16).to_be_bytes()].concat();
	Ok([chunk(b"MThd", &header), chunk(b"MTrk", &track)].concat())
}

struct Reader<'a> {
	bytes: &'a [u8],
	at: usize,
}

impl Reader<'_> {
	fn take(&mut self, count: usize) -> Result<&[u8], String> {
		let end = self.at + count;
		let taken = self.bytes.get(self.at..end).ok_or("the file ends inside an event")?;
		self.at = end;
		Ok(taken)
	}
	fn byte(&mut self) -> Result<u8, String> {
		Ok(self.take(1)?[0])
	}
	fn variable_length(&mut self) -> Result<u32, String> {
		let mut value = 0u32;
		for _ in 0..4 {
			let byte = self.byte()?;
			value = value << 7 | (byte & 0x7f) as u32;
			if byte & 0x80 == 0 {
				return Ok(value);
			}
		}
		Err("a delay is longer than four bytes".into())
	}
}

/// The notes of the tracks of a Standard MIDI File (formats 0 and 1, tempo changes followed), sorted by start
pub fn decode(bytes: &[u8]) -> Result<Vec<i64>, String> {
	let mut file = Reader { bytes, at: 0 };
	if file.take(4).ok() != Some(b"MThd") {
		return Err("not a MIDI file (it does not start with MThd)".into());
	}
	let header_length = u32::from_be_bytes(file.take(4)?.try_into().expect("4 bytes")) as usize;
	let header = file.take(header_length)?;
	if header.len() < 6 {
		return Err("the MIDI header is too short".into());
	}
	let (tracks, division) = (u16::from_be_bytes([header[2], header[3]]), u16::from_be_bytes([header[4], header[5]]));
	if division & 0x8000 != 0 {
		return Err("MIDI files timed in SMPTE frames are not supported, only ticks per quarter note".into());
	}
	let mut tempi = vec![(0u64, MICROSECONDS_PER_QUARTER)];
	let mut messages = vec![]; // (tick, on?, channel, key, velocity)
	for _ in 0..tracks {
		let kind = file.take(4)?.to_vec();
		let length = u32::from_be_bytes(file.take(4)?.try_into().expect("4 bytes")) as usize;
		let body = file.take(length)?;
		if kind != b"MTrk" {
			continue;
		}
		read_track(body, &mut tempi, &mut messages)?;
	}
	tempi.sort_by_key(|(tick, _)| *tick);
	let milliseconds = |tick: u64| {
		let (mut seconds, mut from, mut tempo) = (0.0, 0, MICROSECONDS_PER_QUARTER);
		for &(at, next) in &tempi {
			if at >= tick {
				break;
			}
			seconds += (at - from) as f64 * tempo as f64 / 1e6 / division as f64;
			(from, tempo) = (at, next);
		}
		((seconds + (tick - from) as f64 * tempo as f64 / 1e6 / division as f64) * 1000.0).round() as i64
	};
	messages.sort_by_key(|message| message.0);
	let mut sounding: Vec<(u64, u8, u8, u8)> = vec![];
	let mut notes: Vec<[i64; FIELDS]> = vec![];
	for (tick, on, channel, key, velocity) in messages {
		if on {
			sounding.push((tick, channel, key, velocity));
		} else if let Some(index) = sounding.iter().position(|(_, c, k, _)| (*c, *k) == (channel, key)) {
			let (start, _, _, velocity) = sounding.remove(index);
			let begin = milliseconds(start);
			notes.push([begin, key as i64, milliseconds(tick) - begin, velocity as i64]);
		}
	}
	notes.sort_by_key(|note| (note[0], note[1]));
	Ok(notes.concat())
}

fn read_track(
	body: &[u8],
	tempi: &mut Vec<(u64, u32)>,
	messages: &mut Vec<(u64, bool, u8, u8, u8)>,
) -> Result<(), String> {
	let mut track = Reader { bytes: body, at: 0 };
	let (mut tick, mut status) = (0u64, 0u8);
	while track.at < body.len() {
		tick += track.variable_length()? as u64;
		let first = track.byte()?;
		if first & 0x80 != 0 {
			status = first;
		} else {
			track.at -= 1; // running status: the byte is data
		}
		match status {
			0xff => {
				let (kind, length) = (track.byte()?, track.variable_length()? as usize);
				let data = track.take(length)?;
				if kind == 0x51 && length == 3 {
					tempi.push((tick, u32::from_be_bytes([0, data[0], data[1], data[2]])));
				}
			}
			0xf0 | 0xf7 => {
				let length = track.variable_length()? as usize;
				track.take(length)?;
			}
			_ => {
				let (kind, channel) = (status & 0xf0, status & 0x0f);
				let data = if kind == 0xc0 || kind == 0xd0 { vec![track.byte()?] } else { track.take(2)?.to_vec() };
				match kind {
					NOTE_ON => messages.push((tick, data[1] > 0, channel, data[0], data[1])),
					NOTE_OFF => messages.push((tick, false, channel, data[0], 0)),
					_ => {}
				}
			}
		}
	}
	Ok(())
}

/// A message of up to three bytes to the output `port` ("" the first one), at its voice's moment (card midi-clock):
/// after the sounds queued before it, or at an `at`'s time; headless the line names that place on the voice
pub fn send(port: &str, bytes: &[u8]) -> Result<(), String> {
	if bytes.is_empty() {
		return Err("a MIDI message has at least a status byte".into());
	}
	if !crate::graphics::paint::shows_windows() {
		let hex = bytes.iter().map(|byte| format!("{byte:02x}")).collect::<Vec<_>>();
		let (_, place) = crate::sound::moment();
		let at = if place > 0.0 { format!(" at {place:.3} s") } else { String::new() };
		eprintln!("midi out {port}: {}{at}", hex.join(" "));
		return Ok(());
	}
	crate::host::sandbox::check_process("midi.send")?;
	let (port, bytes) = (port.to_string(), bytes.to_vec());
	crate::sound::on_clock(move || sent(&port, &bytes))
}

/// The message to its output, connected once and kept open for the run
fn sent(port: &str, bytes: &[u8]) -> Result<(), String> {
	let mut outputs = OUTPUTS.lock().map_err(|_| "the MIDI outputs are poisoned")?;
	let name = chosen_port(port, &outputs_of(&midi_output()?), "output")?;
	if !outputs.contains_key(&name) {
		let output = midi_output()?;
		let device = output.ports().into_iter().find(|device| output.port_name(device).ok().as_deref() == Some(&name));
		let device = device.ok_or_else(|| format!("the MIDI output {name} is gone"))?;
		let connection =
			output.connect(&device, CLIENT).map_err(|failure| format!("cannot open MIDI output {name}: {failure}"))?;
		outputs.insert(name.clone(), connection);
	}
	let connection = outputs.get_mut(&name).expect("connected above");
	connection.send(bytes).map_err(|failure| format!("cannot send to MIDI output {name}: {failure}"))
}

/// The names of the MIDI outputs the system has
pub fn outputs() -> Result<Vec<String>, String> {
	crate::host::sandbox::check_process("midi.outputs")?;
	Ok(outputs_of(&midi_output()?))
}

/// The names of the MIDI inputs the system has
pub fn inputs() -> Result<Vec<String>, String> {
	crate::host::sandbox::check_process("midi.inputs")?;
	Ok(inputs_of(&midi_input()?))
}

/// The next message from the input `port` ("" the first one): its bytes, waiting for it; `seconds` > 0 waits that long
/// at most, then `None`. The input stays open for the run, so no message between two calls is lost
pub fn receive(port: &str, seconds: f64) -> Result<Option<Vec<u8>>, String> {
	crate::host::sandbox::check_process("midi.receive")?;
	let mut inputs = INPUTS.lock().map_err(|_| "the MIDI inputs are poisoned")?;
	let name = chosen_port(port, &inputs_of(&midi_input()?), "input")?;
	if !inputs.contains_key(&name) {
		inputs.insert(name.clone(), opened_input(&name)?);
	}
	let messages = &inputs[&name].messages;
	match seconds > 0.0 {
		true => match messages.recv_timeout(std::time::Duration::from_secs_f64(seconds)) {
			Ok(message) => Ok(Some(message)),
			Err(mpsc::RecvTimeoutError::Timeout) => Ok(None),
			Err(mpsc::RecvTimeoutError::Disconnected) => Err(format!("the MIDI input {name} closed")),
		},
		false => messages.recv().map(Some).map_err(|_| format!("the MIDI input {name} closed")),
	}
}

/// An open input whose messages queue up for receive
struct Input {
	_connection: MidiInputConnection<()>,
	messages: mpsc::Receiver<Vec<u8>>,
}

fn opened_input(name: &str) -> Result<Input, String> {
	let input = midi_input()?;
	let device = input.ports().into_iter().find(|device| input.port_name(device).ok().as_deref() == Some(name));
	let device = device.ok_or_else(|| format!("the MIDI input {name} is gone"))?;
	let (queue, messages) = mpsc::channel();
	let forward = move |_stamp: u64, message: &[u8], _: &mut ()| {
		let _ = queue.send(message.to_vec());
	};
	let connection = input
		.connect(&device, CLIENT, forward, ())
		.map_err(|failure| format!("cannot open MIDI input {name}: {failure}"))?;
	Ok(Input { _connection: connection, messages })
}

/// The device named `wanted` (exactly, else the first whose name contains it), "" the first one
fn chosen_port(wanted: &str, names: &[String], direction: &str) -> Result<String, String> {
	let found = match wanted.is_empty() {
		true => names.first(),
		false => names.iter().find(|name| *name == wanted).or_else(|| names.iter().find(|name| name.contains(wanted))),
	};
	found.cloned().ok_or_else(|| match names.is_empty() {
		true => format!("no MIDI {direction} devices found"),
		false => format!("no MIDI {direction} named {wanted}; there are {}", names.join(", ")),
	})
}

fn midi_output() -> Result<MidiOutput, String> {
	MidiOutput::new(CLIENT).map_err(|failure| format!("no MIDI output: {failure}"))
}

fn midi_input() -> Result<MidiInput, String> {
	MidiInput::new(CLIENT).map_err(|failure| format!("no MIDI input: {failure}"))
}

fn outputs_of(output: &MidiOutput) -> Vec<String> {
	output.ports().iter().filter_map(|device| output.port_name(device).ok()).collect()
}

fn inputs_of(input: &MidiInput) -> Vec<String> {
	input.ports().iter().filter_map(|device| input.port_name(device).ok()).collect()
}
