//! The standard library's adapters (notes/stdlib.md section 7, adapter A): the words of lib/<module>.warp that warp
//! cannot write itself call the host words std_pure / std_io (module, member, arguments), answered here natively and
//! by host.js's twin in the browser. Nodes in, a Node out; a failure is the error naming module.member. JSON crosses as
//! for the foreign runtimes (foreign.rs json_of / node_of, host.js plainOfTree / treeOfPlain): null ø, booleans yes/no.

use crate::node::Node;

/// module.member applied to the arguments (a list node); each module's words below, their errors named here
pub fn call(module: &str, member: &str, arguments: &Node) -> Result<Node, String> {
	let arguments = arguments.as_items();
	let answer = match module {
		"json" => json(member, &arguments),
		"file" => file(member, &arguments),
		"hash" => hash(member, &arguments),
		"regex" => regex(member, &arguments),
		"net" => net(member, &arguments),
		"os" => os(member, &arguments),
		"store" => store(member, &arguments),
		// `clipboard.write(text)` (lowering/signals/system_values.rs)
		#[cfg(feature = "native")]
		"clipboard" => match (member, arguments.as_slice()) {
			("write", [text]) => warp_runtime::system_values::write_clipboard(&text_of(text)?).map(|_| Node::Empty),
			_ => no_such_word(&arguments),
		},
		#[cfg(feature = "native")]
		"sound" => sound(member, &arguments),
		#[cfg(feature = "native")]
		"midi" => midi(member, &arguments),
		// the tables of registered classes (lowering/data/database_tables.rs)
		#[cfg(feature = "native")]
		"table" => crate::host::database::call(member, &arguments),
		_ => no_such_word(&arguments),
	};
	answer.map_err(|problem| format!("{module}.{member}: {problem}"))
}

fn no_such_word(arguments: &[Node]) -> Result<Node, String> {
	Err(format!("no such word of {} arguments", arguments.len()))
}

fn text_of(node: &Node) -> Result<String, String> {
	node.text_value().ok_or_else(|| format!("needs a text, got {}", node.serialize().trim()))
}

#[cfg(feature = "native")]
fn number_of(node: &Node) -> Result<f64, String> {
	match node.drop_meta() {
		Node::Number(number) => Ok(f64::from(*number)),
		other => Err(format!("needs a number, got {}", other.serialize().trim())),
	}
}

/// What write puts into a file: a text as it is, any other value as warp writes it (`42`, `[1 2]`)
fn content_of(node: &Node) -> String {
	text_of(node).unwrap_or_else(|_| node.serialize().trim().to_string())
}

fn said(problem: impl std::fmt::Display) -> String {
	problem.to_string()
}

fn json(member: &str, arguments: &[Node]) -> Result<Node, String> {
	match (member, arguments) {
		("parse", [text]) => {
			let text = text_of(text)?;
			let parsed: serde_json::Value = serde_json::from_str(&text)
				.or_else(|_| serde_json::from_str(&crate::node::json_of_json5(&text)))
				.map_err(|problem| format!("{text:?} is no json: {problem}"))?;
			Ok(crate::modules::foreign::node_of(&parsed))
		}
		("to_json", [value]) => Ok(Node::Text(crate::modules::foreign::json_of(value).to_string())),
		// an instance of one of the program's classes is its fields: `{"Point": {"x": 1}}` is `{"x": 1}` (as host.js)
		("to_json", [value, classes]) => {
			let classes: Vec<String> = classes.children().iter().filter_map(|class| text_of(class).ok()).collect();
			Ok(Node::Text(without_class_tags(crate::modules::foreign::json_of(value), &classes).to_string()))
		}
		_ => no_such_word(arguments),
	}
}

fn file(member: &str, arguments: &[Node]) -> Result<Node, String> {
	match (member, arguments) {
		("write", [path, text]) => write_file(&text_of(path)?, content_of(text)),
		("write_bytes", [path, bytes]) => write_file(&text_of(path)?, bytes_of(bytes)?),
		("read_bytes", [path]) => std::fs::read(text_of(path)?)
			.map(|bytes| list_of(bytes.into_iter().map(|byte| Node::int(byte.into()))))
			.map_err(said),
		("append", [path, text]) => {
			use std::io::Write;
			let path = text_of(path)?;
			crate::host::sandbox::check_write(&path)?;
			let mut file = std::fs::OpenOptions::new().append(true).create(true).open(path).map_err(said)?;
			file.write_all(content_of(text).as_bytes()).map(|_| Node::Empty).map_err(said)
		}
		("exists", [path]) => Ok(Node::from(std::path::Path::new(&text_of(path)?).exists())),
		("list", [folder]) => names_in(&text_of(folder)?).map(texts),
		("glob", [pattern]) => globbed(&text_of(pattern)?).map(texts),
		_ => no_such_word(arguments),
	}
}

/// The names of a folder's entries, sorted
fn names_in(folder: &str) -> Result<Vec<String>, String> {
	let mut names: Vec<String> = std::fs::read_dir(folder)
		.map_err(said)?
		.filter_map(|entry| Some(entry.ok()?.file_name().to_string_lossy().into_owned()))
		.collect();
	names.sort();
	Ok(names)
}

/// `glob("src/*.rs")`: the paths matching a pattern whose wildcards `*` and `?` are in its last part; a hidden file
/// only when the pattern's name starts with `.`, as in the shell
fn globbed(pattern: &str) -> Result<Vec<String>, String> {
	let (folder, name, prefix) = match pattern.rsplit_once('/') {
		Some(("", name)) => ("/", name, "/".to_string()),
		Some((folder, name)) => (folder, name, format!("{folder}/")),
		None => (".", pattern, String::new()),
	};
	let wildcards = regex::escape(name).replace(r"\*", "[^/]*").replace(r"\?", "[^/]");
	let matching = regex_of(&format!("^{wildcards}$"))?;
	let shown = |entry: &String| matching.is_match(entry) && (name.starts_with('.') || !entry.starts_with('.'));
	Ok(names_in(folder)?.into_iter().filter(shown).map(|entry| format!("{prefix}{entry}")).collect())
}

fn hash(member: &str, arguments: &[Node]) -> Result<Node, String> {
	match (member, arguments) {
		("sha256", [subject]) => {
			use sha2::Digest;
			Ok(Node::Text(
				sha2::Sha256::digest(content_of(subject).as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect(),
			))
		}
		("crc32", [subject]) => Ok(Node::int(i64::from(crc32fast::hash(content_of(subject).as_bytes())))),
		_ => no_such_word(arguments),
	}
}

fn regex(member: &str, arguments: &[Node]) -> Result<Node, String> {
	let [text, pattern, rest @ ..] = arguments else {
		return no_such_word(arguments);
	};
	let (text, pattern) = (text_of(text)?, regex_of(&text_of(pattern)?)?);
	match (member, rest) {
		("matches", []) => Ok(Node::from(pattern.is_match(&text))),
		("first", []) => Ok(pattern.find(&text).map_or(Node::Empty, |found| Node::Text(found.as_str().to_string()))),
		("all", []) => Ok(texts(pattern.find_iter(&text).map(|found| found.as_str().to_string()))),
		("replace", [replacement]) => {
			Ok(Node::Text(pattern.replace_all(&text, text_of(replacement)?.as_str()).into_owned()))
		}
		_ => no_such_word(arguments),
	}
}

/// `post(url, body)`, `post(url, body, {"x-api-key": key})`: the map's pairs are the request's headers
fn net(member: &str, arguments: &[Node]) -> Result<Node, String> {
	let (url, body, headers) = match (member, arguments) {
		("post", [url, body]) => (url, body, vec![]),
		("post", [url, body, headers]) => (url, body, header_pairs(headers)?),
		_ => return no_such_word(arguments),
	};
	crate::extensions::utils::post_within(&text_of(url)?, &content_of(body), &headers, crate::host::FETCH_TIMEOUT)
		.map(Node::Text)
}

fn os(member: &str, arguments: &[Node]) -> Result<Node, String> {
	match (member, arguments) {
		("env", [name]) => Ok(std::env::var(text_of(name)?).map_or(Node::Empty, Node::Text)),
		("args", []) => Ok(texts(PROGRAM_ARGUMENTS.with(|arguments| arguments.borrow().clone()))),
		("cwd", []) => {
			std::env::current_dir().map(|folder| Node::Text(folder.to_string_lossy().into_owned())).map_err(said)
		}
		("input", prompt) => typed_line(prompt),
		("stdin", []) => std::io::read_to_string(std::io::stdin()).map(Node::Text).map_err(said),
		// `command("ls | wc -l")`: the line run by sh, the explicit opt-in to text as shell (decision use-script)
		#[cfg(feature = "native")]
		("command", [line]) => executed(&["sh".into(), "-c".into(), content_of(line)]).map(Node::Text),
		#[cfg(feature = "native")]
		("exec", [command]) => {
			let command: Vec<String> = command.as_items().iter().map(content_of).collect();
			executed(&command).map(Node::Text)
		}
		_ => no_such_word(arguments),
	}
}

/// `stored theme = "dark"` (lowering/data/stored_values.rs): the values kept by name in a store file
fn store(member: &str, arguments: &[Node]) -> Result<Node, String> {
	match (member, arguments) {
		// the value kept under its name, or the default
		("load", [name, default, file]) => {
			let kept = stored_values(&text_of(file)?)?.remove(&text_of(name)?);
			Ok(kept.map_or_else(|| default.clone(), |value| crate::modules::foreign::node_of(&value)))
		}
		// `stored theme default "dark"`: the default kept from the first run on, so the store lists it
		("declare", [name, default, file]) => {
			let (name, file) = (text_of(name)?, text_of(file)?);
			let mut values = stored_values(&file)?;
			if let Some(kept) = values.get(&name) {
				return Ok(crate::modules::foreign::node_of(kept));
			}
			values.insert(name, crate::modules::foreign::json_of(default));
			save_stored_values(&file, values).map(|_| default.clone())
		}
		("save", [name, value, file]) => {
			let name = text_of(name)?;
			changed_store(file, |values| drop(values.insert(name, crate::modules::foreign::json_of(value))))
		}
		// `delete storage[k]`, `keys(storage)`, `storage` alone
		("remove", [name, file]) => {
			let name = text_of(name)?;
			changed_store(file, |values| drop(values.remove(&name)))
		}
		("all", [file]) => {
			Ok(crate::modules::foreign::node_of(&serde_json::Value::Object(stored_values(&text_of(file)?)?)))
		}
		("names", [file]) => Ok(texts(stored_values(&text_of(file)?)?.into_iter().map(|(name, _)| name))),
		_ => no_such_word(arguments),
	}
}

/// `play "song.mp3"`, `stop_sound`, `fade_sound` (lib/sound.warp, cards sound-library, sound-pro, native-sound)
#[cfg(feature = "native")]
fn sound(member: &str, arguments: &[Node]) -> Result<Node, String> {
	match (member, arguments) {
		("play_file", [path]) => {
			return crate::sound::play_file(&text_of(path)?).map(|handle| Node::int(handle as i64))
		}
		// stop(handle), play's handle; handle 0: all of them
		("stop", [handle]) if number_of(handle)? == 0.0 => crate::sound::stop(),
		("stop", [handle]) => crate::sound::stop_one(number_of(handle)?.max(0.0) as usize)?,
		("fade", [handle, gain, seconds]) => {
			crate::sound::fade(number_of(handle)?.max(0.0) as usize, number_of(gain)?, number_of(seconds)?)?
		}
		("at", [seconds]) => crate::sound::at(number_of(seconds)?),
		// card shared-buses: bus(number, [[name, settings…] …]) its effects (number 0: a new bus), send(bus) the next sounds, returning the bus before
		("bus", [bus, effects]) => {
			let effects = effects.as_items().iter().map(effect_of).collect::<Result<Vec<_>, String>>()?;
			return crate::sound::define_bus(number_of(bus)?.max(0.0) as usize, effects)
				.map(|bus| Node::int(bus as i64));
		}
		("send", [bus]) => return Ok(Node::int(crate::sound::send_to(number_of(bus)?.max(0.0) as usize) as i64)),
		("at_end", []) => crate::sound::at_end(),
		("rest", [seconds]) => crate::sound::rest(number_of(seconds)?),
		("wait", []) => crate::sound::wait(),
		("last", []) => return Ok(Node::int(crate::sound::last_handle() as i64)),
		("moment", []) => return Ok(Node::float(crate::sound::moment_seconds())),
		("queued", []) => return Ok(Node::float(crate::sound::queued_seconds())),
		("render", [path]) => return crate::sound::render(&text_of(path)?).map(Node::float),
		("load", [path]) => {
			return crate::sound::audio_files::load(&text_of(path)?)
				.map(|flat| Node::list(flat.into_iter().map(Node::int).collect()))
		}
		_ => return no_such_word(arguments),
	}
	Ok(Node::Empty)
}

/// `["delay", 0.25, 0.4, 0.5]`: an effect by its name and settings (src/sound/effects.rs Effect::named)
#[cfg(feature = "native")]
fn effect_of(effect: &Node) -> Result<crate::sound::effects::Effect, String> {
	let items = effect.as_items();
	let (name, settings) = items.split_first().ok_or("an effect is [name, settings…]")?;
	let settings = settings.iter().map(number_of).collect::<Result<Vec<_>, String>>()?;
	crate::sound::effects::Effect::named(&text_of(name)?, &settings)
}

/// MIDI (card midi-out, src/sound/midi.rs): notes cross as flat whole numbers [start ms, key, length ms, velocity …]
#[cfg(feature = "native")]
fn midi(member: &str, arguments: &[Node]) -> Result<Node, String> {
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

type StoredValues = serde_json::Map<String, serde_json::Value>;

thread_local! {
	/// The command line arguments after the program file (`warp run prog.warp a b`), what `use os; args` gives
	static PROGRAM_ARGUMENTS: std::cell::RefCell<Vec<String>> = Default::default();
}

/// The arguments the running program gets as `args`
pub fn set_program_arguments(arguments: Vec<String>) {
	PROGRAM_ARGUMENTS.with(|kept| *kept.borrow_mut() = arguments);
}

thread_local! {
	/// The stored values kept in memory while the process runs, by store: of a program without a file (`warp eval`,
	/// tests) and a dev page's under "", the session's (`session[k]`) under its own
	static UNFILED_STORES: std::cell::RefCell<std::collections::HashMap<&'static str, StoredValues>> = Default::default();
}

/// The store in memory a file stands for: of a program without a file, those a `warp dev` page keeps itself (its
/// sessionStorage), the session's, inline code's database; none for a store file
fn unfiled_store(file: &str) -> Option<&'static str> {
	match file {
		"" | crate::stored_values::DEV_STORE => Some(""),
		crate::stored_values::SESSION_STORE => Some(crate::stored_values::SESSION_STORE),
		crate::stored_values::DATABASE_STORE => Some(crate::stored_values::DATABASE_STORE),
		_ => None,
	}
}

/// The stored values in the program's store file (a JSON object by name), none when it does not exist yet
fn stored_values(file: &str) -> Result<StoredValues, String> {
	if let Some(store) = unfiled_store(file) {
		return Ok(UNFILED_STORES.with(|stores| stores.borrow().get(store).cloned().unwrap_or_default()));
	}
	match std::fs::read_to_string(file) {
		Ok(text) => serde_json::from_str(&text).map_err(|problem| format!("{file} holds no stored values: {problem}")),
		Err(problem) if problem.kind() == std::io::ErrorKind::NotFound => Ok(StoredValues::new()),
		Err(problem) => Err(format!("{file}: {problem}")),
	}
}

/// writes a file, creating its missing folders first
fn write_file(path: &str, contents: impl AsRef<[u8]>) -> Result<Node, String> {
	crate::host::sandbox::check_write(path)?;
	let folder = std::path::Path::new(path).parent().filter(|folder| !folder.as_os_str().is_empty());
	folder
		.map_or(Ok(()), std::fs::create_dir_all)
		.and_then(|_| std::fs::write(path, contents))
		.map(|_| Node::Empty)
		.map_err(|problem| format!("{path}: {problem}"))
}

/// The store file's values changed and saved
fn changed_store(file: &Node, change: impl FnOnce(&mut StoredValues)) -> Result<Node, String> {
	let file = text_of(file)?;
	let mut values = stored_values(&file)?;
	change(&mut values);
	save_stored_values(&file, values).map(|_| Node::Empty)
}

fn save_stored_values(file: &str, values: StoredValues) -> Result<(), String> {
	if let Some(store) = unfiled_store(file) {
		UNFILED_STORES.with(|stores| stores.borrow_mut().insert(store, values));
		return Ok(());
	}
	crate::host::sandbox::check_write(file)?;
	let text = serde_json::to_string_pretty(&values).map_err(|problem| problem.to_string())?;
	std::fs::write(file, text).map_err(|problem| format!("{file}: {problem}"))
}

/// What the command (program, arguments…) printed, run without a shell, its trailing newlines dropped as `$(…)` does;
/// a command that fails is an error with what it printed to stderr
#[cfg(feature = "native")]
fn executed(command: &[String]) -> Result<String, String> {
	crate::host::sandbox::check_process("os.exec")?;
	let (program, arguments) = command.split_first().ok_or("needs a command")?;
	let output = std::process::Command::new(program)
		.args(arguments)
		.stdin(std::process::Stdio::null())
		.output()
		.map_err(|problem| format!("cannot run {program}: {problem}"))?;
	if !output.status.success() {
		return Err(format!(
			"{} failed ({}): {}",
			command.join(" "),
			output.status,
			String::from_utf8_lossy(&output.stderr).trim()
		));
	}
	Ok(String::from_utf8_lossy(&output.stdout).trim_end_matches('\n').to_string())
}

/// A line typed on standard input without its line end, ø at the input's end; a prompt is shown first
fn typed_line(prompt: &[Node]) -> Result<Node, String> {
	use std::io::Write;
	if let [prompt] = prompt {
		print!("{}", content_of(prompt));
		std::io::stdout().flush().map_err(said)?;
	}
	let mut line = String::new();
	Ok(match std::io::stdin().read_line(&mut line).map_err(said)? {
		0 => Node::Empty,
		_ => Node::Text(line.trim_end_matches(['\n', '\r']).to_string()),
	})
}

/// The whole numbers of a list argument
#[cfg(feature = "native")]
fn whole_numbers(list: &Node) -> Result<Vec<i64>, String> {
	list.as_items()
		.iter()
		.map(|item| match item.drop_meta() {
			Node::Number(crate::extensions::numbers::Number::Int(value)) => Ok(*value),
			other => Err(format!("needs whole numbers, got {}", other.serialize().trim())),
		})
		.collect()
}

/// A list of texts
fn texts(items: impl IntoIterator<Item = String>) -> Node {
	list_of(items.into_iter().map(Node::Text))
}

fn list_of(items: impl IntoIterator<Item = Node>) -> Node {
	Node::List(items.into_iter().collect(), crate::node::Bracket::Square, crate::node::Separator::Space)
}

/// The bytes a list of ints 0 to 255 stands for
fn bytes_of(list: &Node) -> Result<Vec<u8>, String> {
	list.as_items()
		.iter()
		.map(|item| {
			match item.drop_meta() {
				Node::Number(crate::extensions::numbers::Number::Int(value)) => u8::try_from(*value).ok(),
				_ => None,
			}
			.ok_or_else(|| format!("{} is no byte (0 to 255)", item.serialize().trim()))
		})
		.collect()
}

/// The pattern, if both engines read it alike: Rust's regex has no look-around or backreferences, so JS RegExp may not
/// use them either (host.js checks the same); every other syntax error is the regex crate's own
fn regex_of(pattern: &str) -> Result<regex::Regex, String> {
	if let Some(feature) = unshared_feature(pattern) {
		return Err(format!("{feature} is not in warp's regex (one engine lacks it): {pattern}"));
	}
	regex::Regex::new(pattern).map_err(|problem| problem.to_string())
}

/// look-around `(?=`, `(?!`, `(?<=`, `(?<!` or a backreference `\1`
fn unshared_feature(pattern: &str) -> Option<&'static str> {
	let look_around = ["(?=", "(?!", "(?<=", "(?<!"].iter().any(|opening| pattern.contains(opening));
	let backreference =
		pattern.as_bytes().windows(2).any(|pair| pair[0] == b'\\' && pair[1].is_ascii_digit() && pair[1] != b'0');
	match (look_around, backreference) {
		(true, _) => Some("look-around"),
		(_, true) => Some("a backreference"),
		_ => None,
	}
}

/// A map of texts `{"x-api-key": key}` as (name, value) pairs
fn header_pairs(headers: &Node) -> Result<Vec<(String, String)>, String> {
	let serde_json::Value::Object(pairs) = crate::modules::foreign::json_of(headers) else {
		return Err(format!("headers are a map of texts, got {}", headers.serialize().trim()));
	};
	pairs
		.into_iter()
		.map(|(name, value)| match value {
			serde_json::Value::String(text) => Ok((name, text)),
			other => Err(format!("header {name} needs a text, got {other}")),
		})
		.collect()
}

/// The json with each object `{"Point": {…}}` of a class named in `classes` as its fields `{…}`
fn without_class_tags(value: serde_json::Value, classes: &[String]) -> serde_json::Value {
	use serde_json::Value;
	match value {
		Value::Array(items) => Value::Array(items.into_iter().map(|item| without_class_tags(item, classes)).collect()),
		Value::Object(entries) => match entries.len() == 1
			&& entries.iter().next().is_some_and(|(key, fields)| classes.contains(key) && fields.is_object())
		{
			true => {
				without_class_tags(entries.into_iter().next().map(|(_, fields)| fields).unwrap_or_default(), classes)
			}
			false => Value::Object(
				entries.into_iter().map(|(key, entry)| (key, without_class_tags(entry, classes))).collect(),
			),
		},
		other => other,
	}
}
