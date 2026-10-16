//! The command line's flags: hints and warning levels, strictness, the sandbox, compile flags
use super::*;

pub(crate) const COMPILE_FLAGS: [&str; 7] =
	[EXE_FLAG, WASM_FLAG, AOT_FLAG, SITE_FLAG, WIT_FLAG, COMPONENT_FLAG, WAGI_FLAG];
/// Where earlier versions remembered "got it", in the current folder: their `ack:` lines are adopted
pub(crate) const LOCAL_ACKNOWLEDGEMENTS_FILES: [&str; 2] = [".warp-answers", ACKNOWLEDGEMENTS_FILE];
/// `warp fuel <steps> …` (an option word like `fast`) and its flag spelling `--fuel <steps>`
pub(crate) const FUEL_WORD: &str = "fuel";
pub(crate) const FUEL_FLAG: &str = "--fuel";
/// Never prompt "got it?" after a warning or note (as in CI or a pipe)
pub(crate) const NO_ASK_FLAG: &str = "--no-ask";
/// Hints and notes (`prefer ^ over **`) are shown by default; `--no-hints` or WARP_HINTS=0 hide them (card hints-toggle)
pub(crate) const NO_HINTS_FLAG: &str = "--no-hints";
pub(crate) const HINTS_VARIABLE: &str = "WARP_HINTS";
/// `--warnings=<level>`: print diagnostics from that level up (hint, weak, normal, severe); `--no-hints` is weak
pub(crate) const WARNINGS_FLAG: &str = "--warnings";
/// `--strict` (normal and severe warnings are errors, as `use strict`), `--strict=<level>` from another level
pub(crate) const STRICT_FLAG: &str = "--strict";
/// `warp fast …`: every program as if it began with `use fast_math` (lib/fast_math.warp: sin cos exp pow atan2 as wasm
/// polynomials)
pub(crate) use warp::words::FAST_WORD;
/// The last line of a run that showed hints while nobody chose to see them (neither the flag nor WARP_HINTS was given)
pub(crate) const HIDE_HINTS_TIP: &str = "hide hints with: warp --no-hints (or WARP_HINTS=0)";

/// The argument of `warp <command> <argument>`; an assigned command word (`warp 'lower = "abc"; lower'`) starts a
/// program instead (card top-level)
pub(crate) fn command_argument<'a>(arg_string: &'a str, command: &str) -> Option<&'a str> {
	let argument = arg_string.strip_prefix(command.trim_end())?.strip_prefix(' ')?;
	let assigned = ["=", ":="].iter().any(|operator| argument.trim_start().starts_with(operator));
	(!assigned).then_some(argument)
}

/// `--fuel <steps>`, `fast`, `--strict`, `--no-ask`, `--no-hints` take effect and leave the arguments; the answers file is read
#[cfg(not(test))]
pub(crate) fn apply_flags(args: &mut Vec<String>) {
	// `warp fuel <steps> <file|code …>` or `--fuel <steps>`: every run counts fuel steps instead of stopping at the time
	// limit (decision fuel-metering; env WARP_FUEL)
	let fuel_word_at = (args.len() > 3 && args[1] == FUEL_WORD).then_some(1);
	if let Some(flag) = args.iter().position(|arg| arg == FUEL_FLAG).or(fuel_word_at) {
		match args.get(flag + 1).and_then(|steps| steps.replace('_', "").parse::<u64>().ok()) {
			Some(steps) => util::set_fuel_budget(steps),
			None => {
				eprintln!("{} needs a number of steps, e.g. warp fuel 100000000000 app.warp", args[flag]);
				std::process::exit(2);
			}
		}
		args.drain(flag..flag + 2);
	}

	// `warp fast <file|code|command …>`: an option word (decision cli-words), only before something to run
	if args.len() > 2 && args[1] == FAST_WORD {
		args.remove(1);
		warp::pipeline::set_fast_math(true);
	}

	// `--strict[=level]`: warnings of that level and up are errors (as `use strict` in the program)
	if let Some(level) = level_flag(args, STRICT_FLAG, diagnostic::STRICT_LEVEL) {
		diagnostic::set_warning_mode(diagnostic::WarningMode::Strict(level));
	}

	let no_hints = args.iter().position(|arg| arg == NO_HINTS_FLAG).map(|flag| args.remove(flag)).is_some();
	let hints_variable = env::var(HINTS_VARIABLE).ok();
	let hints_wanted = hints_variable.as_deref().is_none_or(|value| !matches!(value, "0" | "false" | "no" | "off"));
	let hints_shown = hints_wanted && !no_hints;
	let fewest_shown = if hints_shown { diagnostic::Level::Hint } else { diagnostic::Level::Weak };
	let shown = level_flag(args, WARNINGS_FLAG, fewest_shown).unwrap_or(fewest_shown);
	diagnostic::show_from(shown);
	if shown == diagnostic::Level::Hint && hints_variable.is_none() {
		tip_hiding_hints_at_exit();
	}

	// "got it?" is asked on the terminal after a warning or note unless nobody is there to answer
	let no_ask = args.iter().position(|arg| arg == NO_ASK_FLAG).map(|flag| args.remove(flag)).is_some();
	if !no_ask && env::var_os("CI").is_none() && io::stdin().is_terminal() && io::stderr().is_terminal() {
		diagnostic::set_acknowledger(Some(std::rc::Rc::new(diagnostic::TerminalAcknowledger)));
	}
	let acknowledgements = dirs_home().join(ACKNOWLEDGEMENTS_FILE);
	for local_file in LOCAL_ACKNOWLEDGEMENTS_FILES {
		diagnostic::adopt_acknowledgements(local_file, &acknowledgements);
	}
	diagnostic::use_acknowledgements_file(acknowledgements);
}

/// The level of `--flag=<level>` or `--flag <level>`, `bare` for the flag alone, taken out of the arguments; exits on an
/// unknown level
#[cfg(not(test))]
pub(crate) fn level_flag(args: &mut Vec<String>, flag: &str, bare: diagnostic::Level) -> Option<diagnostic::Level> {
	let at = args.iter().position(|arg| arg == flag || arg.starts_with(&format!("{flag}=")))?;
	let given = match args.remove(at).split_once('=') {
		Some((_, name)) => Some(name.to_string()),
		None => args.get(at).filter(|name| diagnostic::Level::named(name).is_some()).cloned().inspect(|_| {
			args.remove(at);
		}),
	};
	let Some(name) = given else {
		return Some(bare);
	};
	diagnostic::Level::named(&name).or_else(|| {
		let names: Vec<&str> = diagnostic::Level::ALL.iter().map(|level| level.name()).collect();
		eprintln!("{flag} takes a level: {}", names.join(", "));
		std::process::exit(2);
	})
}

/// After the run (also one ending in process::exit), one line telling how to hide the hints it showed
#[cfg(not(test))]
pub(crate) fn tip_hiding_hints_at_exit() {
	extern "C" fn tip() {
		if diagnostic::any_hint_printed() {
			eprintln!("{}", diagnostic::paint(diagnostic::Color::Gray, HIDE_HINTS_TIP));
		}
	}
	extern "C" {
		fn atexit(callback: extern "C" fn()) -> i32;
	}
	// SAFETY: atexit only stores the function pointer; tip touches no state torn down before exit handlers run
	unsafe { atexit(tip) };
}

/// The compile flags `arguments` start with, in any order, and the arguments after them
pub(crate) fn leading_flags(mut arguments: String) -> (Vec<&'static str>, String) {
	let mut flags = vec![];
	while let Some(flag) = COMPILE_FLAGS.into_iter().find(|flag| arguments.split_whitespace().next() == Some(flag)) {
		flags.push(flag);
		arguments = arguments[flag.len()..].trim_start().to_string();
	}
	(flags, arguments)
}
