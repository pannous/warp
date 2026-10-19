//! A crash of a warp built from a source checkout files a card on the project's to-do board (`todo add`, column Now),
//! so crashes met by whoever runs warp reach the board, not only those of tests. Each panic location and message files
//! one card ever, across all checkouts and worktrees (listed in ~/.cache/warp/crash_cards.txt; the board's
//! urgent-card-watch also folds a card into an open one with the same fingerprint). The card names the checkout and
//! branch the crashing build came from, so a crash in a worker's worktree build goes to that worker. Builds without
//! the checkout (brew, releases) and CI runs file nothing; WARP_CRASH_CARDS=0 (set by tests/queue.sh) turns it off.
//! A build older than its checkout's src/ or lib/ files nothing either: it says so and asks for a rebuild.

use std::io::Write;
use std::panic::PanicHookInfo;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};

const SOURCE_CHECKOUT: &str = env!("CARGO_MANIFEST_DIR");
/// Below $HOME: shared by the user's checkout and every worktree, which re-filed the same panic each (2026-10-12)
const FILED_LIST: &str = ".cache/warp/crash_cards.txt";
const OFF_SWITCH: &str = "WARP_CRASH_CARDS";
const BOARD_COLUMN: &str = "Now";
const TITLE_CHARS: usize = 110;
const BACKTRACE_CHARS: usize = 20_000;
/// Backtrace sources of std and the dependencies, not of warp
const FOREIGN_SOURCES: [&str; 3] = ["/rustc/", "/.cargo/", "/.rustup/"];
/// The backtrace frame where the panic starts unwinding: the frames after it lead to the crash
const UNWIND_FRAME: &str = "rust_begin_unwind";
/// The commit this warp was built from (build.rs)
const BUILT_COMMIT: Option<&str> = option_env!("WARP_COMMIT");
/// What a warp binary is made of: compiled in from src/, read at run time from lib/ (lib/compiler/*.warp)
const BUILD_SOURCES: [&str; 2] = ["src", "lib"];

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
	if let Some(head) = newer_checkout(checkout) {
		return eprintln!(
			"warp: no crash card: this build is of {}, its checkout {SOURCE_CHECKOUT} is at {head} with other src/ or \
			 lib/ files; rebuild (scripts/own-warp.sh) and run it again",
			BUILT_COMMIT.unwrap_or_default()
		);
	}
	let message = panic_message(info);
	let headline = message.lines().next().unwrap_or_default();
	let backtrace: String =
		std::backtrace::Backtrace::force_capture().to_string().chars().take(BACKTRACE_CHARS).collect();
	let panic_place =
		|| info.location().map_or("an unknown place".to_string(), |place| format!("{}:{}", place.file(), place.line()));
	let location = first_warp_frame(&backtrace).unwrap_or_else(panic_place);
	// without the line: an edit above the crash moves it, the crash stays the same
	let fingerprint = format!("{} {headline}", location.split(" at ").next().unwrap_or_default());
	let filed_list = filed_list(checkout);
	if std::fs::read_to_string(&filed_list).unwrap_or_default().lines().any(|line| line == fingerprint) {
		return eprintln!("warp: this crash has a card on the to-do board already ({})", filed_list.display());
	}
	let title: String = format!("crash in {location}: {headline}").chars().take(TITLE_CHARS).collect();
	let command_line = std::env::args().collect::<Vec<_>>().join(" ");
	// todo urgent reads the Fingerprint and Build lines (~/dev/bin/todo, CRASH_FINGERPRINT and CRASH_BUILD)
	let body = format!(
		"Filed by warp's panic hook (src/diagnostics/crash_card.rs).\n\nFingerprint: `{fingerprint}`\nBuild: `{SOURCE_CHECKOUT}` on branch `{}`\n\n`{command_line}`\n\n```\n{message}\n```\n\n```\n{backtrace}\n```",
		current_branch(checkout)
	);
	let mut todo = Command::new("todo");
	todo.args(["add", &title, BOARD_COLUMN, "-b", &body])
		.current_dir(checkout)
		.stdin(Stdio::null())
		.stdout(Stdio::null());
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

/// The checkout's HEAD when its src/ or lib/ differ from the commit this warp was built from: a crash of such a stale
/// build may be fixed already, or come from lib/compiler files newer than its parser (card crash-4)
fn newer_checkout(checkout: &Path) -> Option<String> {
	let built = BUILT_COMMIT?;
	let head = git(checkout, &["rev-parse", "--short=9", "HEAD"])?;
	let unchanged =
		|| git(checkout, &[&["diff", "--quiet", built, &head, "--"][..], &BUILD_SOURCES].concat()).is_some();
	(head != built && !unchanged()).then_some(head)
}

/// git's output in the checkout, None when it fails
fn git(checkout: &Path, arguments: &[&str]) -> Option<String> {
	let output =
		Command::new("git").args(arguments).current_dir(checkout).stdin(Stdio::null()).stderr(Stdio::null()).output();
	output
		.ok()
		.filter(|output| output.status.success())
		.map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn filed_list(checkout: &Path) -> PathBuf {
	std::env::var_os("HOME")
		.map_or_else(|| checkout.join("data/crash_cards.txt"), |home| Path::new(&home).join(FILED_LIST))
}

/// The branch checked out where this warp was built (a worker's worktree, or main in the user's checkout)
fn current_branch(checkout: &Path) -> String {
	git(checkout, &["branch", "--show-current"]).unwrap_or_default()
}

/// The innermost frame in warp's own code, `name at src/file.rs:line`: a panic inside std (a failed print) names its
/// caller, not the std line. The frames above the unwind are the panic hooks (this file's, main.rs's closed-pipe
/// hook), never the crash: card crash-closure was titled `{closure#0} at src/main.rs:171`
pub fn first_warp_frame(backtrace: &str) -> Option<String> {
	let lines: Vec<&str> = backtrace.lines().collect();
	let crash_frames = lines.iter().rposition(|line| line.contains(UNWIND_FRAME)).unwrap_or(0);
	lines.iter().enumerate().skip(crash_frames).find_map(|(index, line)| {
		let (number, name) = line.trim().split_once(": ")?;
		number.parse::<usize>().ok()?;
		let source = lines.get(index + 1).and_then(|next| next.trim().strip_prefix("at "));
		let foreign = |path: &str| FOREIGN_SOURCES.iter().any(|marker| path.contains(marker));
		let own_source =
			source.filter(|path| !foreign(path)).and_then(|path| path.find("/src/").map(|start| &path[start + 1..]));
		match own_source {
			Some(path) if !path.starts_with("src/diagnostics/crash_card.rs") => Some(format!("{name} at {path}")),
			_ if name.contains("warp::") && !name.contains("crash_card") => Some(name.to_string()),
			_ => None,
		}
	})
}

fn panic_message(info: &PanicHookInfo) -> String {
	let payload = info.payload();
	payload
		.downcast_ref::<&str>()
		.map(|text| text.to_string())
		.or_else(|| payload.downcast_ref::<String>().cloned())
		.unwrap_or_else(|| "a panic without a message".to_string())
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
