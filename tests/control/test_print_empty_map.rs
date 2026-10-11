//! card print-empty-map: an empty map is ø, the empty list, and joins as ø wherever a text is made of it: `print({})`
//! was "not a joinable item" when the program had no nested list to write (list_join without list_text)
use crate::is;

#[test]
fn an_empty_map_joins_as_empty() {
	is!("print({})", warp::Node::Empty);
	is!("{} as text", "ø");
	is!("\"a\" + {}", "aø");
	is!("x = {}; \"a\" + x", "aø");
}

#[cfg(all(unix, feature = "native"))]
#[test]
fn print_of_an_empty_map_prints_empty() {
	let printed = crate::common::printed("print({})");
	assert_eq!(printed.lines().filter(|line| !line.starts_with('»')).collect::<Vec<_>>(), ["ø"]);
}
