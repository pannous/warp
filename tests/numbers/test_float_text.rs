//! A float computed at run time has a text: print, `str(x)`, `x as text`, `"a" + x` and join show at most 15
//! significant digits, positional from 1e-5 up to 1e15, else with an exponent
use crate::is;

/// The float `value` computed at run time (a host random number times zero keeps the compiler from folding it)
fn run_time(value: &str) -> String {
	format!("x = random() * 0 + {value}; ")
}

#[test]
fn test_float_as_text() {
	for (value, text) in [("1.5", "1.5"), ("0.1", "0.1"), ("0.30000000000000004", "0.3"), ("-2.25", "-2.25"), ("100.0", "100"),
		("1e20", "1e20"), ("1.5e-7", "1.5e-7"), ("0.00012", "0.00012"), ("12345678901234.5", "12345678901234.5")] {
		is!(&format!("{}str(x)", run_time(value)), text);
	}
}

#[test]
fn test_float_joins_texts() {
	is!(&format!("{}\"v: \" + x", run_time("2.5")), "v: 2.5");
	is!(&format!("{}[x, 1, \"a\"].join(\" \")", run_time("2.5")), "2.5 1 a");
	// print gives nothing (issue #18): what it writes
	#[cfg(feature = "native")]
	assert!(crate::common::printed(&format!("{}print x", run_time("0.25"))).starts_with("0.25\n"));
}
