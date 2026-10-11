//! A crash of a warp built from a source checkout files a card on the project's to-do board (`todo add`, column Now),
//! so crashes met by whoever runs warp reach the board, not only those of tests. Each panic location and message files
//! one card ever (listed in data/crash_cards.txt of that checkout). Builds without the checkout (brew, releases)
//! and CI runs file nothing; WARP_CRASH_CARDS=0 (set by tests/queue.sh) turns it off.

use std::io::Write;
use std::panic::PanicHookInfo;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};

const SOURCE_CHECKOUT: &str = env!("CARGO_MANIFEST_DIR");
const FILED_LIST: &str = "data/crash_cards.txt";
const OFF_SWITCH: &str = "WARP_CRASH_CARDS";
const BOARD_COLUMN: &str = "Now";
const TITLE_CHARS: usize = 110;
const BACKTRACE_CHARS: usize = 20_000;
/// Backtrace sources of std and the dependencies, not of warp
const FOREIGN_SOURCES: [&str; 3] = ["/rustc/", "/.cargo/", "/.rustup/"];

/// One card per run: a panic while panicking ("panic in a function that cannot unwind") tells nothing new
static FILED: AtomicBool = AtomicBool::new(false);

/// Keep the default report on stderr and file a card after it
pub fn install() {
	let default_hook = std::panic::take_hook();
	std::panic::set_hook(Box::new(move |info| {
		default_hook(info);
		file_card(info);
	}));
}

fn file_card(info: &PanicHookInfo) {
	let checkout = Path::new(SOURCE_CHECKOUT);
	let switched_off = std::env::var(OFF_SWITCH).is_ok_and(|switch| switch == "0") || std::env::var_os("CI").is_some();
	if switched_off || !checkout.join(".git").exists() || FILED.swap(true, Ordering::SeqCst) {
		return;
	}
	let message = panic_message(info);
	let headline = message.lines().next().unwrap_or_default();
	let backtrace: String = std::backtrace::Backtrace::force_capture().to_string().chars().take(BACKTRACE_CHARS).collect();
	let panic_place = || info.location().map_or("an unknown place".to_string(), |place| format!("{}:{}", place.file(), place.line()));
	let location = first_warp_frame(&backtrace).unwrap_or_else(panic_place);
	// without the line: an edit above the crash moves it, the crash stays the same
	let fingerprint = format!("{} {headline}", location.split(" at ").next().unwrap_or_default());
	let filed_list = checkout.join(FILED_LIST);
	if std::fs::read_to_string(&filed_list).unwrap_or_default().lines().any(|line| line == fingerprint) {
		return eprintln!("warp: this crash has a card on the to-do board already ({FILED_LIST})");
	}
	let title: String = format!("crash in {location}: {headline}").chars().take(TITLE_CHARS).collect();
	let command_line = std::env::args().collect::<Vec<_>>().join(" ");
	let body = format!("Filed by warp's panic hook (src/crash_card.rs).\n\n`{command_line}`\n\n```\n{message}\n```\n\n```\n{backtrace}\n```");
	let mut todo = Command::new("todo");
	todo.args(["add", &title, BOARD_COLUMN, "-b", &body]).current_dir(checkout).stdin(Stdio::null()).stdout(Stdio::null());
	// its own process group: it outlives this process, which may abort right after the hook
	#[cfg(unix)]
	std::os::unix::process::CommandExt::process_group(&mut todo, 0);
	match todo.spawn() {
		Ok(_) => {
			remember(&filed_list, &fingerprint);
			eprintln!("warp: filing a card for this crash on the to-do board: {title}");
		}
		Err(failure) => eprintln!("warp: could not file a card for this crash, `todo` failed: {failure}"),
	}
}

/// The innermost frame in warp's own code, `name at src/file.rs:line`: a panic inside std (a failed print) names its
/// caller, not the std line
fn first_warp_frame(backtrace: &str) -> Option<String> {
	let lines: Vec<&str> = backtrace.lines().collect();
	lines.iter().enumerate().find_map(|(index, line)| {
		let (number, name) = line.trim().split_once(": ")?;
		number.parse::<usize>().ok()?;
		let source = lines.get(index + 1).and_then(|next| next.trim().strip_prefix("at "));
		let foreign = |path: &str| FOREIGN_SOURCES.iter().any(|marker| path.contains(marker));
		let own_source = source.filter(|path| !foreign(path)).and_then(|path| path.find("/src/").map(|start| &path[start + 1..]));
		match own_source {
			Some(path) if !path.starts_with("src/crash_card.rs") => Some(format!("{name} at {path}")),
			_ if name.contains("warp::") && !name.contains("crash_card") => Some(name.to_string()),
			_ => None,
		}
	})
}

fn panic_message(info: &PanicHookInfo) -> String {
	let payload = info.payload();
	payload.downcast_ref::<&str>().map(|text| text.to_string()).or_else(|| payload.downcast_ref::<String>().cloned()).unwrap_or_else(|| "a panic without a message".to_string())
}

fn remember(filed_list: &Path, fingerprint: &str) {
	let appended = filed_list.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| {
		let mut list = std::fs::OpenOptions::new().create(true).append(true).open(filed_list)?;
		writeln!(list, "{fingerprint}")
	});
	if let Err(failure) = appended {
		eprintln!("warp: cannot note the filed crash card in {}: {failure}", filed_list.display());
	}
}
