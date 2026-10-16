// decision units-library (card standalone-math-units): the SI prefixes, which dimensions take them and the long names of
// conversion targets are lines of lib/units.warp's unit_definitions block, not Rust tables
use crate::common::fails_with;
use crate::is;

#[test]
fn each_prefix_line_is_a_power_of_ten() {
	is!("1 Gs == 1000000000 s", true);
	is!("1000 µg == 1 mg", true);
	is!("1 dam == 10 m", true);
}

#[test]
fn only_an_si_dimension_takes_prefixes() {
	is!("1 kK == 1000 K", true);
	fails_with("3 kpx", "undefined: kpx");
}

#[test]
fn a_name_line_is_a_conversion_target() {
	is!("2 hours in minutes == 120 min", true);
	is!("3 feet in inches == 36 inch", true);
}

// reviewer: a program writes the same lines as lib/units.warp, except dimensions
#[test]
fn a_program_writes_its_own_prefix_and_name_lines() {
	is!("prefix x := 10^4\n3 xm in km == 30 km", true);
	is!("unit fur := 201 m\nname furlong := fur\n3 fur in furlongs == 3 fur", true);
	fails_with("prefix T := 10^12\n3 Tm", "prefix T: a prefix is a power of ten");
	fails_with("SI dimension length meter\n1", "(dimensions) are lib/units.warp's");
}

// reviewer veto: `name` and `prefix` stay function names; a line is a unit line where its right side reads as one
#[test]
fn a_definition_named_name_or_prefix_stays_a_definition() {
	is!("name p := p.first; name {first: \"Bo\"}", "Bo");
	is!("prefix s := s#1; prefix \"ab\"", 'a');
}

// decision quantity-naming: `quantity` replaces `dimension` for length, speed, density; `dimension` is ambiguous (a
// quantity, a tensor's axis, rank or shape), so the old word gets a hint asking which
#[test]
fn quantity_lines_are_the_librarys_and_dimension_asks_which() {
	fails_with("SI quantity length meter\n1", "quantities (dimensions) are lib/units.warp's");
	warp::normalize::clear_shown_hints();
	let (_, hints) = warp::normalize::capture_hints(|| warp::wasm_emitter::eval("SI dimension length meter\n1"));
	assert!(
		hints.iter().any(|hint| hint.original == "dimension"
			&& hint.canonical == "quantity"
			&& hint.reason.contains("quantity, axis, rank or shape?")),
		"{hints:?}"
	);
}
