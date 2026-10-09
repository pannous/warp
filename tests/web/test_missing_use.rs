// A word of a standard module used without its `use` (card clickable-hint): the error offers a fix that adds the `use`
// line at the top, and the page's "add: use math" button applies it
use std::collections::HashSet;
use warp::web::evaluate;

const MISSING_USE: &str = "square 3";
const WITH_USE: &str = "use math\nsquare 3";

#[test]
fn a_missing_use_offers_to_add_it() {
	let report = evaluate(MISSING_USE, HashSet::new());
	assert_eq!(report["error"], true, "{report}");
	let fix = &report["errors"][0]["fixes"][0];
	assert_eq!(fix["label"], "add: use math");
	assert_eq!(fix["edits"][0]["start"], 0);
	assert_eq!(fix["edits"][0]["end"], 0);
	assert_eq!(fix["edits"][0]["replacement"], "use math\n");
}

#[test]
fn the_added_use_makes_it_run() {
	assert_eq!(evaluate(WITH_USE, HashSet::new())["value"], "9");
}
