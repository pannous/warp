//! MIDI (card midi-out, notes/sound.md layer 7): Standard MIDI Files and live output.
//!
//! A note crosses to the host as four whole numbers, start in milliseconds, key (0..127), length in milliseconds and
//! velocity (1..127); a file is a list of them flattened. Files are format 0, 480 ticks a quarter at 120 beats per
//! minute (960 ticks a second), so a millisecond is a whole number of ticks to within 5%: ticks are rounded, not kept.
//! Live output goes to `sendmidi` when the user may hear it; headless (tests, CI, WARP_NO_WINDOW) only a
//! `midi out <port>: 90 3c 64` line on stderr, as sound prints its WAV. A message goes out at its voice's moment like a
//! sound (card midi-clock): after the sounds before it, or at an `at`'s time. Live input has no backend yet, natively or
//! in the playground (its Web MIDI input stays on the page, out of the program's Worker): it fails loudly (card midi-receive).

use std::process::Command;

const TICKS_PER_QUARTER: u32 = 480;
const MICROSECONDS_PER_QUARTER: u32 = 500_000;
const TICKS_PER_SECOND: f64 = TICKS_PER_QUARTER as f64 * 1_000_000.0 / MICROSECONDS_PER_QUARTER as f64;
const NOTE_ON: u8 = 0x90;
const NOTE_OFF: u8 = 0x80;
const SENDER: &str = "sendmidi";
const SENDER_INSTALL: &str = "brew install gbevin/tools/sendmidi";
/// A note's four numbers in the flat list
pub const FIELDS: usize = 4;

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
	let hex = bytes.iter().map(|byte| format!("{byte:02x}")).collect::<Vec<_>>();
	if !crate::graphics::paint::shows_windows() {
		let (_, place) = crate::sound::moment();
		let at = if place > 0.0 { format!(" at {place:.3} s") } else { String::new() };
		eprintln!("midi out {port}: {}{at}", hex.join(" "));
		return Ok(());
	}
	crate::host::sandbox::check_process("midi.send")?;
	let port = port.to_string();
	crate::sound::on_clock(move || sent(&port, &hex))
}

/// The message's hex bytes handed to the sender now
fn sent(port: &str, hex: &[String]) -> Result<(), String> {
	let port = if port.is_empty() {
		outputs()?.into_iter().next().ok_or("no MIDI output devices found")?
	} else {
		port.to_string()
	};
	let status =
		Command::new(SENDER).args(["dev", &port, "hex"]).args(hex).status().map_err(|failure| no_backend(&failure))?;
	status.success().then_some(()).ok_or_else(|| format!("{SENDER} failed ({status}) sending to {port}"))
}

/// The names of the MIDI outputs the system has
pub fn outputs() -> Result<Vec<String>, String> {
	crate::host::sandbox::check_process("midi.outputs")?;
	let listed = Command::new(SENDER).arg("list").output().map_err(|failure| no_backend(&failure))?;
	Ok(String::from_utf8_lossy(&listed.stdout)
		.lines()
		.map(|line| line.trim().trim_matches('"').to_string())
		.filter(|line| !line.is_empty())
		.collect())
}

fn no_backend(failure: &std::io::Error) -> String {
	format!("no MIDI output: {SENDER} could not run ({failure}); install it with `{SENDER_INSTALL}` (the playground uses Web MIDI)")
}

/// There is no native MIDI input: failing loudly beats a program that waits for messages that cannot come
pub fn receive() -> Result<(), String> {
	Err("MIDI input is not available natively yet (no CoreMIDI binding in this build), nor in the playground (card midi-receive)".into())
}
