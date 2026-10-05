//! A value of another runtime without a JSON form stays there behind an id, a handle `{$handle: id, type, text}`; a
//! member of a variable holding one, or of a chained call, asks that runtime (src/foreign.rs, notes/stdlib_connectors.md)
use warp::is;

#[test]
fn test_a_js_handle() {
	is!("use js Intl; f = Intl.NumberFormat(\"en\"); f.format(1234.5)", "1,234.5");
}

#[cfg(feature = "native")] // python3
#[test]
fn test_python_handles() {
	is!("use python \"datetime\"; d = datetime.date(2020, 1, 2); d.isoformat()", "2020-01-02");
	is!("use python \"datetime\"; d = datetime.date(2020, 1, 2); d.year", 2020);
	is!("use python \"pathlib\"; p = pathlib.Path(\"a/b.txt\"); p.suffix", ".txt");
	is!("use python \"datetime\"; datetime.date(2020, 1, 2).isoformat()", "2020-01-02");
}

#[cfg(feature = "native")]
#[test]
fn test_a_handle_goes_back_as_an_argument() {
	is!("use python \"datetime\"; d = datetime.date(2020, 1, 2); use python \"copy\"; e = copy.copy(d); e.month", 1);
}

#[cfg(feature = "native")]
#[test]
fn test_python_numbers_stay_numbers() {
	is!("use python \"fractions\"; fractions.Fraction(1, 2).numerator", 1);
	is!("use python \"decimal\"; d = decimal.Decimal(\"1.5\"); d.is_signed()", 0);
}
