//! `warp --sandbox` (card sandbox-exec): a sandboxed program starts no process, writes no file and reaches no network.
//! The capability check in pipeline.rs sees only the words a program calls directly (Capability is also where a word
//! is imported from, so fetch stays Host): the words below are refused by name, at compile time where they show
//! (`fetch`) and at run time where a library word hides them (`exec sh"…"` is os.exec through std_io).
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};

const FLAG: &str = "warp --sandbox";

/// What a refused word reaches outside the program
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
	Process,
	FileWrite,
	Network,
}

impl Reach {
	pub fn name(self) -> &'static str {
		match self {
			Reach::Process => "process",
			Reach::FileWrite => "file-write",
			Reach::Network => "network",
		}
	}
}

/// The host words a sandboxed program may not call
const REFUSED_HOST_WORDS: [(&str, Reach); 3] = [
	(crate::host::FETCH_START, Reach::Network),
	("fetch", Reach::Network),
	("fetch_within", Reach::Network),
];
/// The standard library's adapter words (src/std_adapters.rs) a sandboxed program may not call; `*` is every member
const REFUSED_STD_WORDS: [(&str, &str, Reach); 7] = [
	("os", "exec", Reach::Process),
	("file", "write", Reach::FileWrite),
	("file", "write_bytes", Reach::FileWrite),
	("file", "append", Reach::FileWrite),
	("midi", "write_file", Reach::FileWrite),
	("sound", "render", Reach::FileWrite),
	("net", "*", Reach::Network),
];

thread_local! {
	static SANDBOXED: Cell<bool> = const { Cell::new(false) };
}
/// Set once by `warp --sandbox` for its whole run: the threads tasks run on are sandboxed too
static PROCESS_SANDBOXED: AtomicBool = AtomicBool::new(false);

/// `run` sandboxed on this thread
pub fn confined<T>(run: impl FnOnce() -> T) -> T {
	crate::util::scoped(&SANDBOXED, true, run)
}

/// Every thread of this process sandboxed, for good
pub fn confine_process() {
	PROCESS_SANDBOXED.store(true, Ordering::SeqCst);
}

pub fn is_sandboxed() -> bool {
	PROCESS_SANDBOXED.load(Ordering::SeqCst) || SANDBOXED.with(Cell::get)
}

/// The loud refusal of `word`, which needs `capability`
pub fn denial(word: &str, capability: &str) -> String {
	format!("capability denied: {word} needs the {capability} capability, which {FLAG} does not grant")
}

/// The refusal of host word `word` when sandboxed
pub fn check_host_word(word: &str) -> Result<(), String> {
	refuse(word, refused_host_word(word))
}

/// The refusal of the adapter word `module.member` when sandboxed
pub fn check_std_word(module: &str, member: &str) -> Result<(), String> {
	let reach = REFUSED_STD_WORDS
		.iter()
		.find(|(refused, word, _)| *refused == module && (*word == member || *word == "*"))
		.map(|(_, _, reach)| *reach);
	refuse(&format!("{module}.{member}"), reach)
}

/// The first refused host word of `words` (a program's externals), when sandboxed
pub fn first_refused<'a>(words: impl IntoIterator<Item = &'a String>) -> Option<String> {
	words.into_iter().find_map(|word| check_host_word(word).err())
}

fn refused_host_word(word: &str) -> Option<Reach> {
	REFUSED_HOST_WORDS
		.iter()
		.find(|(refused, _)| *refused == word)
		.map(|(_, reach)| *reach)
}

fn refuse(word: &str, reach: Option<Reach>) -> Result<(), String> {
	match reach {
		Some(reach) if is_sandboxed() => Err(denial(word, reach.name())),
		_ => Ok(()),
	}
}
