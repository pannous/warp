//! `now!` in data, `friends{peter james created:now!}`: the instant the run reads, printed as a date; the block it ran
//! in gave the folded time back as a Rust value, printed "warp::time::calendar::Time" (card data-run-now,
//! wiki/partial-evaluation.md)
use crate::is;

#[test]
fn a_forced_now_is_a_time_value() {
	is!("t = now!; t.year >= 2026", true);
	is!("x = {created: now!, n: 1}; x.created.year >= 2026", true); // card map-now-bang: read when the run reads it
}

#[cfg(feature = "native")]
#[test]
fn now_in_data_prints_as_a_date() {
	use crate::common::printed;
	let output = printed("friends{peter james created:now!}\nprint friends");
	assert!(output.starts_with("{peter james created:20"), "{output}");
	assert!(!printed("x = now!\nprint x").contains("calendar"));
}
