// Hints are shown by default and end with one line on hiding them; `--no-hints` or WARP_HINTS=0 hide them; warnings
// stay (user 2026-10-09, card hints-toggle)
#![cfg(feature = "native")]

const HINTED: &str = "2 ** 3";
const HIDE_TIP: &str = "hide hints with: warp --no-hints";

fn stderr_of(arguments: &[&str], hints_variable: Option<&str>) -> String {
	let mut command = crate::common::warp_command();
	command.env_remove("WARP_HINTS").args(arguments);
	if let Some(value) = hints_variable {
		command.env("WARP_HINTS", value);
	}
	String::from_utf8_lossy(&command.output().expect("warp runs").stderr).into_owned()
}

#[test]
fn a_plain_run_shows_its_hints_and_how_to_hide_them() {
	let errors = stderr_of(&["--no-ask", "eval", HINTED], None);
	assert!(errors.contains("hint 1:3: prefer ^ over **"), "{errors}");
	assert!(errors.trim_end().lines().last().is_some_and(|last| last.contains(HIDE_TIP)), "{errors}");
	assert_eq!(errors.matches(HIDE_TIP).count(), 1, "{errors}");
}

#[test]
fn a_run_without_hints_has_no_tip() {
	assert!(!stderr_of(&["--no-ask", "eval", "2 ^ 3"], None).contains(HIDE_TIP));
}

#[test]
fn no_hints_hides_them() {
	assert!(!stderr_of(&["--no-ask", "--no-hints", "eval", HINTED], None).contains("hint"));
	assert!(!stderr_of(&["--no-ask", "eval", HINTED], Some("0")).contains("hint"));
}

#[test]
fn hints_chosen_explicitly_need_no_tip() {
	let errors = stderr_of(&["--no-ask", "eval", HINTED], Some("1"));
	assert!(errors.contains("prefer ^ over **") && !errors.contains(HIDE_TIP), "{errors}");
}
