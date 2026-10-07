// card whenever-constant (user, 2026-10-07): `age = alice.age; whenever age > 40 {…}` listens on a copy nothing writes
// again, so it never fires: a strong warning with the did-you-mean `age := alice.age` (a derived value follows alice.age)
use crate::common::{warnings_of, warns_with};

const NEVER_FIRES: &str = "never fires";

#[test]
fn a_listener_on_a_copy_warns_with_the_derived_value() {
	let copy = "alice = {age: 30}; age = alice.age; whenever age > 40 { print \"old\" }; alice.age = 41; age";
	warns_with(copy, 30, NEVER_FIRES);
	warns_with(copy, 30, "age := alice.age");
	warns_with("limit = 3; once limit > 5 { print \"over\" }; limit", 3, NEVER_FIRES);
}

#[test]
fn a_listener_on_a_written_or_derived_value_does_not_warn() {
	for code in [
		"alice = {age: 30}; age := alice.age; whenever age > 40 { print \"old\" }; alice.age = 41; age",
		"x = 0; whenever x > 3 { print \"big\" }; x = 5; x",
		"n = 1; whenever n > 2 { print \"n\" }; n++; n",
	] {
		let (_, warnings) = warnings_of(code);
		assert!(!warnings.iter().any(|warning| warning.contains(NEVER_FIRES)), "{code}: {warnings:?}");
	}
}
