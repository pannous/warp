//! The system's live MIDI devices through midir (CoreMIDI), macOS only for now like the sound engine (Cargo.toml: Linux
//! would link ALSA); src/sound/midi.rs has the stand-in elsewhere, which finds no devices.

use super::midi::chosen_port;
use midir::{MidiIO, MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use std::collections::HashMap;
use std::sync::{mpsc, Arc, LazyLock, Mutex};

/// The name the system shows for warp's MIDI client and its connections
const CLIENT: &str = "warp";
/// The outputs opened by name, kept open for the run
static OUTPUTS: LazyLock<Mutex<HashMap<String, MidiOutputConnection>>> = LazyLock::new(Default::default);
/// The inputs opened by name, their messages queued
static INPUTS: LazyLock<Mutex<HashMap<String, Input>>> = LazyLock::new(Default::default);

/// The message to its output, connected once and kept open for the run
pub fn send(port: &str, bytes: &[u8]) -> Result<(), String> {
	let mut outputs = OUTPUTS.lock().map_err(|_| "the MIDI outputs are poisoned")?;
	let name = chosen_port(port, &port_names(&midi_output()?), "output")?;
	if !outputs.contains_key(&name) {
		let output = midi_output()?;
		let device = port_named(&output, &name, "output")?;
		let connection =
			output.connect(&device, CLIENT).map_err(|failure| format!("cannot open MIDI output {name}: {failure}"))?;
		outputs.insert(name.clone(), connection);
	}
	let connection = outputs.get_mut(&name).expect("connected above");
	connection.send(bytes).map_err(|failure| format!("cannot send to MIDI output {name}: {failure}"))
}

pub fn output_names() -> Result<Vec<String>, String> {
	Ok(port_names(&midi_output()?))
}

pub fn input_names() -> Result<Vec<String>, String> {
	Ok(port_names(&midi_input()?))
}

/// The next message from the input `port`, as midi::receive describes it
pub fn receive(port: &str, seconds: f64) -> Result<Option<Vec<u8>>, String> {
	let (name, messages) = {
		let mut inputs = INPUTS.lock().map_err(|_| "the MIDI inputs are poisoned")?;
		let name = chosen_port(port, &port_names(&midi_input()?), "input")?;
		if !inputs.contains_key(&name) {
			inputs.insert(name.clone(), opened_input(&name)?);
		}
		let messages = inputs[&name].messages.clone();
		(name, messages)
	};
	let messages = messages.lock().map_err(|_| format!("the MIDI input {name} is poisoned"))?;
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
	messages: Arc<Mutex<mpsc::Receiver<Vec<u8>>>>,
}

fn opened_input(name: &str) -> Result<Input, String> {
	let input = midi_input()?;
	let device = port_named(&input, name, "input")?;
	let (queue, messages) = mpsc::channel();
	let forward = move |_stamp: u64, message: &[u8], _: &mut ()| {
		let _ = queue.send(message.to_vec());
	};
	let connection = input
		.connect(&device, CLIENT, forward, ())
		.map_err(|failure| format!("cannot open MIDI input {name}: {failure}"))?;
	Ok(Input { _connection: connection, messages: Arc::new(Mutex::new(messages)) })
}

fn midi_output() -> Result<MidiOutput, String> {
	MidiOutput::new(CLIENT).map_err(|failure| format!("no MIDI output: {failure}"))
}

fn midi_input() -> Result<MidiInput, String> {
	MidiInput::new(CLIENT).map_err(|failure| format!("no MIDI input: {failure}"))
}

fn port_names(io: &impl MidiIO) -> Vec<String> {
	io.ports().iter().filter_map(|device| io.port_name(device).ok()).collect()
}

fn port_named<T: MidiIO>(io: &T, name: &str, direction: &str) -> Result<T::Port, String> {
	let device = io.ports().into_iter().find(|device| io.port_name(device).ok().as_deref() == Some(name));
	device.ok_or_else(|| format!("the MIDI {direction} {name} is gone"))
}
