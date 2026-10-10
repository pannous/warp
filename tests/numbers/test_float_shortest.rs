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
