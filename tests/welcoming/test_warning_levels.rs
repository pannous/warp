//! One diagnostic system (card warning-levels): every hint and warning has a level, hint < weak < normal < severe.
//! What prints is a threshold (`--warnings=<level>`, `--no-hints` shows from weak); what fails is another
//! (`use strict` and `--strict` fail from normal, `--strict=<level>`). Hints never fail.
use warp::errors::diagnostic::{with_warning_mode, Level, WarningMode, STRICT_LEVEL};
use warp::normalize::{capture_hints, set_hint_mode, HintMode};
use warp::wasm_emitter::eval;
use warp::Node;

const WEAK_WARNING: &str = "square(x) := x * x; xs = [1, 2, 3]; @parallel square xs";
#[cfg(feature = "native")] // only the warp binary's runs read it
const SEVERE_WARNING: &str = "n = 0\non key { if key == ESCAPEE { n += 1 } }\nn";
#[cfg(feature = "native")]
const NORMAL_WARNING: &str = "\"a\" + warning(\"careful\")";
const HINTED: &str = "one plus two";

#[test]
fn levels_are_ordered_and_named() {
	assert!(Level::Hint < Level::Weak && Level::Weak < Level::Normal && Level::Normal < Level::Severe);
	for level in Level::ALL {
		assert_eq!(Level::named(level.name()), Some(level));
	}
	assert_eq!(Level::named("strong"), None);
	assert_eq!(STRICT_LEVEL, Level::Normal);
}

#[test]
fn strict_fails_from_normal_and_the_rust_error_mode_on_every_warning() {
	assert!(!WarningMode::Strict(STRICT_LEVEL).fails(Level::Weak));
	assert!(WarningMode::Strict(STRICT_LEVEL).fails(Level::Normal));
	assert!(WarningMode::Error.fails(Level::Weak));
	for mode in [WarningMode::Warn, WarningMode::Quiet, WarningMode::Error, WarningMode::Strict(Level::Hint)] {
		assert!(!mode.fails(Level::Hint), "{mode:?}: hints never fail");
	}
	assert!(!matches!(eval(&format!("use strict; {WEAK_WARNING}")), Node::Error(_)));
	assert!(matches!(with_warning_mode(WarningMode::Error, || eval(WEAK_WARNING)), Node::Error(_)));
}

#[test]
fn capture_hints_still_returns_the_hints() {
	set_hint_mode(HintMode::Always);
	let (_, hints) = capture_hints(|| eval(HINTED));
	assert!(!hints.is_empty());
}

#[cfg(feature = "native")] // runs the warp binary
fn cli(args: &[&str]) -> String {
	let output = crate::common::warp_command().args(args).env_remove("WARP_HINTS").output().expect("warp runs");
	String::from_utf8_lossy(&output.stdout).to_string() + &String::from_utf8_lossy(&output.stderr)
}

#[test]
#[cfg(feature = "native")]
fn the_cli_labels_levels_and_shows_from_its_threshold() {
	assert!(cli(&[SEVERE_WARNING]).contains("warning (severe):"));
	assert!(cli(&[NORMAL_WARNING]).contains("warning: careful"));
	assert!(!cli(&["--warnings=severe", NORMAL_WARNING]).contains("careful"));
	assert!(cli(&["--warnings=severe", SEVERE_WARNING]).contains("warning (severe):"));
	assert!(cli(&["--no-hints", WEAK_WARNING]).contains("warning (weak):"));
	assert!(cli(&["--strict=severe", NORMAL_WARNING]).contains("warning: careful"));
	assert!(cli(&["--strict", NORMAL_WARNING]).contains("Error(\"careful\")"));
}

#[test]
#[cfg(feature = "native")]
fn no_hints_hides_only_the_hints() {
	let hint = "prefer";
	assert!(cli(&[HINTED]).contains(hint));
	assert!(!cli(&["--no-hints", HINTED]).contains(hint));
	assert!(!cli(&["--warnings=weak", HINTED]).contains(hint));
}
