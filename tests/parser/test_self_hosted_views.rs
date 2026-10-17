//! card warp-pipeline-suite: the self-hosted pipeline (src/compiler/self_hosted.rs) hands each program to the warp
//! stages as its view and takes the node back from the view they give; the node of a view has that view again
use crate::common::twins::sources;
use warp::compiler::node_views::{handled_plain_view, node_of_handled_view};
use warp::warp_parser::parse;
use warp::Node;

/// Characters of the first difference shown
const SHOWN: usize = 300;

#[test]
fn the_node_of_each_sources_view_has_that_view() {
	let differing: Vec<(String, String)> = sources()
		.iter()
		.filter_map(|path| {
			let (view, handles) = handled_plain_view(&parse(&std::fs::read_to_string(path).unwrap()));
			let (again, _) = handled_plain_view(&node_of_handled_view(&view, &handles));
			(again != view).then(|| (path.display().to_string(), first_difference(&again, &view)))
		})
		.collect();
	if let Some((path, difference)) = differing.first() {
		let shown: String = difference.chars().take(SHOWN).collect();
		panic!("{} sources come back from their view otherwise, first {path}: {shown}", differing.len());
	}
}

/// The first part where the views differ, both sides with their kinds (a list's bracket and separator too)
fn first_difference(again: &Node, view: &Node) -> String {
	match (again, view) {
		(Node::List(items, bracket, separator), Node::List(others, other_bracket, other_separator))
			if items.len() == others.len() =>
		{
			if bracket != other_bracket || separator != other_separator {
				return format!("{bracket}/{separator:?} again, {other_bracket}/{other_separator:?} in the view");
			}
			let mut pairs = items.iter().zip(others);
			pairs
				.find(|(item, other)| item != other)
				.map_or_else(String::new, |(item, other)| first_difference(item, other))
		}
		_ => format!("{again:?} ({}) again, {view:?} ({}) in the view", again.kind(), view.kind()),
	}
}
