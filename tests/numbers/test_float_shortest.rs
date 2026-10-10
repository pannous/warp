//! A float prints its shortest digits that read back, as JavaScript and Python write it (user decision float-text):
//! at run time and folded alike, in print, `str(x)` and `"a" + x`
use crate::is;

const SHORTEST_TEXTS: [(&str, &str); 6] = [("0.30000000000000004", "0.30000000000000004"), ("1.0 / 3", "0.3333333333333333"),
	("0.1", "0.1"), ("1.5e-7", "1.5e-7"), ("2.0 / 3", "0.6666666666666666"), ("123456789.123", "123456789.123")];

#[test]
fn test_run_time_float_prints_shortest() {
	for (value, text) in SHORTEST_TEXTS {
		is!(&format!("x = random() * 0 + {value}; str(x)"), text);
	}
}

/// A float scaled for its digits may round across a half: x · 10^17 is …333.5 for 0.1 / 3, whose digits are …333
#[test]
fn test_run_time_float_rounded_across_a_half() {
	is!("x = random() * 0 + 0.1; str(x / 3)", "0.03333333333333333");
}

/// A float no 16 digits read back as gets its 17 digits exactly (Dekker's product); tiny and huge floats their
/// shortest digits too
#[test]
fn test_run_time_float_needing_17_digits() {
	is!("x = random() * 0 + 73.0; str(x / 7)", "10.428571428571429");
	for text in ["0.7999999999999999", "1.4285714285714285e-11", "1.7636684142857142e34", "1.7976931348623157e308", "3.110878728344063e-302", "6.62607015e-34", "1.5e-30", "5e-324", "2.2250738585072014e-308"] {
		is!(&format!("x = random() * 0 + {text}; str(x)"), text);
	}
}

#[test]
fn test_folded_float_prints_shortest() {
	for (value, text) in SHORTEST_TEXTS {
		is!(&format!("str({value})"), text);
		is!(&format!("\"a\" + ({value})"), format!("a{text}").as_str());
	}
}

#[cfg(feature = "native")]
#[test]
fn test_print_float_shortest() {
	assert!(crate::common::printed("print 1/3.0").starts_with("0.3333333333333333\n"));
	assert!(crate::common::printed("x = random() * 0 + 0.30000000000000004; print x").starts_with("0.30000000000000004\n"));
}
