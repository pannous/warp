//! wiki variable.md "cross-referencing from neighbor scope": inside one block a key reads a sibling's data by its path,
//! `{ colors:{red:(1 0 0)} circle:{radius:5 color: colors.red} }` (the data is the scope)
use warp::is;
use warp::wasp_parser::parse;

#[test]
fn test_a_sibling_path_inside_a_block() {
	is!("{ colors:{red:(1 0 0)} circle:{radius:5 color: colors.red} }.circle.color", parse("(1 0 0)"));
	is!("{ a:{x:2} b: a.x }.b", 2);
	is!("{ a:[1 2 3] b: a#2 }.b", 2);
}

#[test]
fn test_the_top_level_form_still_reads_it() {
	is!("colors:{red:(1 0 0)}; circle:{color: colors.red}; circle.color", parse("(1 0 0)"));
}
