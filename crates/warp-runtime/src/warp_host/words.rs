//! The host's std_io (lib/os.warp, lib/file.warp): the words of the std modules a program's runtime cannot give itself
//! over WASI. A failing word answers the program an Error, `os.command: … failed (exit status: 3): …`, as warp-rust's
//! std_adapters.rs does.
use crate::nodes::{self, Builders, Node};
use crate::{cache, Host};
use std::io::{BufRead, Write};
use std::process::{Command, Stdio};
use warp_runtime::gc_values::{parts_of, TEXT_HEAP};
use warp_runtime::host_words::HOST_LIBRARY;
use wasmtime::{AnyRef, Caller, Extern, Linker, Result, Rooted};

type Ref = Option<Rooted<AnyRef>>;

pub fn link(linker: &mut Linker<Host>) -> Result<()> {
	linker.func_wrap(
		HOST_LIBRARY,
		"std_io",
		|mut caller: Caller<'_, Host>, module: Ref, member: Ref, arguments: Ref| {
			let builders = Builders::of(|name| caller.get_export(name))
				.ok_or_else(|| wasmtime::Error::msg("std_io of a program without its node constructors"))?;
			let mut store = caller.as_context_mut();
			let mut read = |node: Ref| nodes::read(&wasmtime::Val::AnyRef(node), &mut store, Some(builders.memory()));
			let (module, member) = (read(module)?.content(), read(member)?.content());
			let raw = arguments;
			let arguments = read(arguments)?.items();
			// keep gives the value it keeps: the program's own, kept or not
			let given = if (module.as_str(), member.as_str()) == ("os", "keep") {
				parts_of(&wasmtime::Val::AnyRef(raw), &mut store)?.get(1).copied()
			} else {
				None
			};
			let answer = std_io(&module, &member, arguments, store.data_mut())
				.unwrap_or_else(|problem| Node::Error(format!("{module}.{member}: {problem}").into_bytes()));
			match given {
				Some(value) => Ok(value.unwrap_anyref().copied()),
				None => Ok(builders.build(&answer, &mut store)?.unwrap_anyref().copied()),
			}
		},
	)?;
	link_memory_words(linker)
}

/// The words of compiler.wasm (built by warp-rust) that pass texts through linear memory: (ptr, len) in, (ptr, len) of
/// the answer out, (ptr, -len) of a failure's reason
fn link_memory_words(linker: &mut Linker<Host>) -> Result<()> {
	linker.func_wrap(HOST_LIBRARY, "read", |mut caller: Caller<'_, Host>, pointer: i32, length: i32| {
		answered(&mut caller, pointer, length, |path| {
			std::fs::read(path).map_err(|problem| format!("read {path} failed: {problem}"))
		})
	})?;
	linker.func_wrap(HOST_LIBRARY, "fetch", |mut caller: Caller<'_, Host>, pointer: i32, length: i32| {
		answered(&mut caller, pointer, length, |url| fetched(url, 30_000))
	})?;
	linker.func_wrap(
		HOST_LIBRARY,
		"fetch_within",
		|mut caller: Caller<'_, Host>, pointer: i32, length: i32, within: i64| {
			answered(&mut caller, pointer, length, |url| fetched(url, within))
		},
	)?;
	linker.func_wrap(HOST_LIBRARY, "warn", |mut caller: Caller<'_, Host>, pointer: i32, length: i32| {
		if let Some(memory) = caller.get_export("memory").and_then(Extern::into_memory) {
			let mut message = vec![0; length as u32 as usize];
			if memory.read(&caller, pointer as u32 as usize, &mut message).is_ok() {
				eprintln!("warning: {}", String::from_utf8_lossy(&message));
			}
		}
	})?;
	Ok(())
}

/// The body at a URL, by curl
fn fetched(url: &str, within_ms: i64) -> Result<Vec<u8>, String> {
	let seconds = format!("{:.3}", within_ms.max(1) as f64 / 1000.0);
	let output = output_of(&["curl", "-fsSL", "--max-time", &seconds, url].map(String::from))?;
	match output.status.success() {
		true => Ok(output.stdout),
		false => Err(format!("fetch {url} failed: {}", String::from_utf8_lossy(&output.stderr).trim())),
	}
}

fn answered(
	caller: &mut Caller<'_, Host>,
	pointer: i32,
	length: i32,
	answer: impl FnOnce(&str) -> Result<Vec<u8>, String>,
) -> (i32, i32) {
	let Some(memory) = caller.get_export("memory").and_then(Extern::into_memory) else { return (0, 0) };
	let heap = caller.get_export(TEXT_HEAP).and_then(Extern::into_global);
	let mut asked = vec![0; length as u32 as usize];
	let answer = match memory.read(&*caller, pointer as u32 as usize, &mut asked) {
		Ok(()) => answer(&String::from_utf8_lossy(&asked)),
		Err(problem) => Err(problem.to_string()),
	};
	let (bytes, sign) = match &answer {
		Ok(bytes) => (bytes.as_slice(), 1),
		Err(reason) => (reason.as_bytes(), -1),
	};
	match nodes::placed(memory, heap, bytes, &mut caller.as_context_mut()) {
		Ok(place) => (place as i32, sign * bytes.len() as i32),
		Err(_) => (0, 0),
	}
}

use wasmtime::AsContextMut;

fn std_io(module: &str, member: &str, arguments: Vec<Node>, host: &mut Host) -> Result<Node, String> {
	let text = |node: &Node| node.content();
	match (module, member, arguments.as_slice()) {
		("os", "env", [name]) => Ok(std::env::var(text(name)).map_or(Node::Empty, Node::text)),
		("os", "args", []) => Ok(Node::List(nodes::SQUARE_LIST, host.args.iter().skip(1).map(Node::text).collect())),
		("os", "cwd", []) => std::env::current_dir().map(|folder| Node::text(folder.to_string_lossy())).map_err(said),
		("os", "input", prompt) => typed_line(prompt.first().map(text)),
		("os", "stdin", []) => std::io::read_to_string(std::io::stdin()).map(Node::text).map_err(said),
		("os", "kept", [name]) => Ok(cache::kept(&host.keeper, &text(name)).unwrap_or(Node::Empty)),
		("os", "keep", [name, value]) => {
			cache::keep(&host.keeper, &text(name), value);
			Ok(value.clone())
		}
		// `command("ls | wc -l")`: the line run by sh, the explicit opt-in to text as shell (decision use-script)
		("os", "command", [line]) => printed(&["sh".into(), "-c".into(), text(line)]),
		("os", "exec", [command]) => printed(&command.clone().items().iter().map(text).collect::<Vec<_>>()),
		("os", "run", [command]) => ran(&command.clone().items().iter().map(text).collect::<Vec<_>>()),
		("file", "read", [path]) => {
			std::fs::read(text(path)).map(Node::Text).map_err(|problem| text(path) + ": " + &said(problem))
		}
		("file", "exists", [path]) => Ok(Node::Bool(std::path::Path::new(&text(path)).exists())),
		("file", "write", [path, content]) => {
			std::fs::write(text(path), text(content)).map_err(said)?;
			Ok(content.clone())
		}
		("file", "write_bytes", [path, bytes]) => {
			let bytes = bytes.clone().items().into_iter().map(|byte| match byte {
				Node::Int(byte) => u8::try_from(byte).map_err(|_| format!("{byte} is no byte (0 to 255)")),
				other => Err(format!("{} is no byte (0 to 255)", other.shown())),
			});
			std::fs::write(text(path), bytes.collect::<Result<Vec<u8>, String>>()?).map_err(said)?;
			Ok(Node::Empty)
		}
		("file", "read_bytes", [path]) => {
			let bytes = std::fs::read(text(path)).map_err(said)?;
			Ok(Node::List(nodes::SQUARE_LIST, bytes.into_iter().map(|byte| Node::Int(byte.into())).collect()))
		}
		("file", "append", [path, content]) => {
			let mut file = std::fs::OpenOptions::new().append(true).create(true).open(text(path)).map_err(said)?;
			file.write_all(text(content).as_bytes()).map_err(said)?;
			Ok(content.clone())
		}
		_ => Err(format!("warp-host has no {module}.{member} of {} arguments", arguments.len())),
	}
}

fn said(problem: std::io::Error) -> String {
	problem.to_string()
}

/// What the command (program, arguments…) printed, run without a shell, its trailing newlines dropped as `$(…)` does;
/// a command that fails is an error with what it printed to stderr
fn printed(command: &[String]) -> Result<Node, String> {
	let output = output_of(command)?;
	if !output.status.success() {
		let stderr = String::from_utf8_lossy(&output.stderr);
		return Err(format!("{} failed ({}): {}", command.join(" "), output.status, stderr.trim()));
	}
	Ok(Node::text(trimmed(&output.stdout)))
}

/// `run(["sh", "-c", "…"])`: the command's {status, out, err}, its outputs' trailing newlines dropped; a failing
/// command is no error, its status says so (killed by a signal: 128 + the signal, as sh says)
fn ran(command: &[String]) -> Result<Node, String> {
	let output = output_of(command)?;
	#[cfg(unix)]
	let signal = std::os::unix::process::ExitStatusExt::signal(&output.status).map(|signal| 128 + signal);
	#[cfg(not(unix))]
	let signal = None;
	let status = output.status.code().or(signal).unwrap_or(-1);
	Ok(Node::map(vec![
		("status", Node::Int(i64::from(status))),
		("out", Node::text(trimmed(&output.stdout))),
		("err", Node::text(trimmed(&output.stderr))),
	]))
}

fn output_of(command: &[String]) -> Result<std::process::Output, String> {
	let (program, arguments) = command.split_first().ok_or("needs a command")?;
	Command::new(program)
		.args(arguments)
		.stdin(Stdio::null())
		.output()
		.map_err(|problem| format!("cannot run {program}: {problem}"))
}

fn trimmed(bytes: &[u8]) -> String {
	String::from_utf8_lossy(bytes).trim_end_matches('\n').to_string()
}

/// A line typed on standard input without its line end, ø at the input's end; a prompt is shown first
fn typed_line(prompt: Option<String>) -> Result<Node, String> {
	if let Some(prompt) = prompt {
		print!("{prompt}");
		std::io::stdout().flush().map_err(said)?;
	}
	let mut line = String::new();
	match std::io::stdin().lock().read_line(&mut line).map_err(said)? {
		0 => Ok(Node::Empty),
		_ => Ok(Node::text(line.trim_end_matches(['\n', '\r']))),
	}
}
