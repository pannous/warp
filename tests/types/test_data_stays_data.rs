//! Data stays data (decision data-stays-data): a bare word as a field's value is the word, `friends{peter james
//! created:now}` prints `created:now`; only `now!` and `${…}` evaluate (card data-bare)
#[cfg(feature = "native")]
use crate::common::printed;
use crate::is;

#[test]
fn a_bare_now_in_data_is_the_word() {
	is!("x = {created: now}\nx.created", warp::Node::Symbol("now".into()));
	is!("t = now; t.year >= 2026", true);
}

#[cfg(feature = "native")]
#[test]
fn now_prints_as_written_and_now_forced_as_a_date() {
	assert_eq!(printed("friends{peter james created:now}\nprint friends"), "{peter james created:now}\n");
	assert!(printed("friends{peter james created:now!}\nprint friends").starts_with("{peter james created:20"));
}

/// Card map-now-bang: `now!` in a map literal is the time the run reads, as in tagged data, not a kept block
#[test]
fn now_forced_in_a_map_is_the_time() {
	is!("x = {created: now!}\nx.created.year >= 2026", true);
	is!("x = {created: now!, n: 1}\nx.n", 1);
}
