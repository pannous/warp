//! The sound module's host words natively (src/host/native.rs links them): sound_samples, play_signal and
//! play_blocks calling the program's functions back for each sample or block

use crate::host::tasks::task_failure;
use crate::host::{calling_back, linear_doubles, list_values, module_memory, number_of, HostNode, HostState};
use crate::node::Node;
use wasmtime::{Caller, Extern};

/// sound_samples(samples, count, rate) natively: a WAV file, played unless headless (src/sound/mod.rs)
pub(crate) fn sound(
	mut caller: Caller<'_, HostState>,
	samples: HostNode,
	count: i64,
	rate: i64,
	channels: i64,
) -> wasmtime::Result<()> {
	let memory = module_memory(&mut caller, "sound")?;
	let values = list_values(&mut caller, memory, samples, count.max(0) as usize)?
		.map_err(|other| task_failure(format!("sound needs a list of samples, got {}", other.serialize())))?;
	crate::sound::sound(&values, rate.clamp(1, u32::MAX as i64) as u32, channels.clamp(1, 2) as u16)
		.map_err(task_failure)
}

/// play_signal(f, seconds) (sound.rs play_signal): the program's f(t: float) called for each sample, in stereo a
/// pair [left, right] of them; its handle
pub(crate) fn played_signal(caller: &mut Caller<'_, HostState>, arguments: &Node) -> wasmtime::Result<Node> {
	let ([functions, seconds, rate], word) =
		(sound_arguments(arguments, "play_signal(f, seconds) needs a function and its seconds")?, "play_signal");
	let signals = channels_of(&functions, word)?
		.iter()
		.map(|function| {
			called_back(
				caller,
				function,
				word,
				"of the time in seconds to a number, as hum(t) := sin(2 * pi * 440 * t)",
			)
		})
		.collect::<wasmtime::Result<Vec<_>>>()?;
	let (seconds, rate) = seconds_and_rate(&seconds, &rate)?;
	let handle = calling_back(word, |kept| {
		crate::sound::play_signal(seconds, rate, signals.len() as u16, |channel, time| {
			signals[channel].call(&mut *caller, time).map_err(&mut *kept)
		})
	})?;
	Ok(Node::int(handle as i64))
}

/// play_blocks(fill, block, seconds) (lib/sound.warp): the program's `fill(t: float)` called for each block, t its first
/// sample's time, which fills the program's linear float arrays at `blocks` (one, or left and right); its handle
pub(crate) fn played_blocks(caller: &mut Caller<'_, HostState>, arguments: &Node) -> wasmtime::Result<Node> {
	let ([function, blocks, seconds, rate], word) =
		(sound_arguments(arguments, "play_blocks(fill, seconds) needs a function and its seconds")?, "play_blocks");
	let fill = called_back(
		caller,
		&function,
		word,
		"of a block of samples and its time, as hum(block, t) := for i in 1 to #block { block#i = … }",
	)?;
	let (seconds, rate) = seconds_and_rate(&seconds, &rate)?;
	let blocks = channels_of(&blocks, word)?
		.iter()
		.map(|block| number_of(block, &task_failure).map(|address| f64::from(address) as i64))
		.collect::<wasmtime::Result<Vec<_>>>()?;
	let size = linear_doubles(caller, blocks[0], &task_failure)?.len();
	let handle = calling_back(word, |kept| {
		crate::sound::play_blocks(seconds, rate, size, blocks.len() as u16, |first, _| {
			fill.call(&mut *caller, first as f64 / rate as f64).map_err(&mut *kept)?;
			blocks.iter().map(|&block| linear_doubles(caller, block, &task_failure).map_err(&mut *kept)).collect()
		})
	})?;
	Ok(Node::int(handle as i64))
}

/// A sound word's one value or stereo pair [left, right] of them
fn channels_of(node: &Node, word: &str) -> wasmtime::Result<Vec<Node>> {
	let channels = match node.drop_meta() {
		Node::List(items, _, _) => items.clone(),
		single => vec![single.clone()],
	};
	match channels.len() {
		1 | 2 => Ok(channels),
		count => Err(task_failure(format!("{word}: {count} channels; one, or a pair [left, right] for stereo"))),
	}
}

fn sound_arguments<const COUNT: usize>(arguments: &Node, usage: &str) -> wasmtime::Result<[Node; COUNT]> {
	arguments.children().try_into().map_err(|_| task_failure(usage.into()))
}

/// The program's function of a float to a float a sound word calls back
fn called_back(
	caller: &mut Caller<'_, HostState>,
	function: &Node,
	word: &str,
	shape: &str,
) -> wasmtime::Result<wasmtime::TypedFunc<f64, f64>> {
	let name = function.name();
	let not_a_signal = || task_failure(format!("{word}: {name} is no function {shape}"));
	caller
		.get_export(&name)
		.and_then(Extern::into_func)
		.ok_or_else(not_a_signal)?
		.typed::<f64, f64>(&*caller)
		.map_err(|_| not_a_signal())
}

fn seconds_and_rate(seconds: &Node, rate: &Node) -> wasmtime::Result<(f64, u32)> {
	let [seconds, rate] = [seconds, rate].map(|number| number_of(number, &task_failure).map(f64::from));
	Ok((seconds?, rate?.clamp(1.0, u32::MAX as f64) as u32))
}
