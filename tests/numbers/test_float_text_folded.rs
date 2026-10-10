//! A float the compiler knows prints as the run-time float_text writes it (test_float_text):
//! one way, the shortest digits that read back (user decision float-text), positional from 1e-5 up to 1e15, else with an exponent
use crate::is;

const FLOAT_TEXTS: [(&str, &str); 6] =
	[("1.5", "1.5"), ("0.30000000000000004", "0.30000000000000004"), ("-2.25", "-2.25"), ("1.5e-7", "1.5e-7"), ("0.00012", "0.00012"), ("1.0 / 3", "0.3333333333333333")];

#[test]
fn test_folded_float_joins_text_as_at_run_time() {
	for (value, text) in FLOAT_TEXTS {
		is!(&format!("\"a\" + ({value})"), format!("a{text}").as_str());
		is!(&format!("str({value})"), text);
	}
}

#[cfg(feature = "native")]
#[test]
fn test_printed_float_literal_as_float_text() {
	assert!(crate::common::printed("print 1.5e-7").starts_with("1.5e-7\n"));
	assert!(crate::common::printed("print 0.30000000000000004").starts_with("0.30000000000000004\n"));
}
