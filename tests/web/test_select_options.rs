//! A list inside `select{}` gives its options, one per item; a map item gives value and label; a bound select marks the
//! chosen one (card select-shorthand)
use warp::wasm_emitter::eval;
use warp::web::markup::to_html;

fn html_of(code: &str) -> String {
	to_html(&eval(code))
}

#[test]
fn a_list_gives_the_options() {
	assert_eq!(
		html_of("select{ aria-label: \"fruit\"; [\"apple\" \"pear\"] }"),
		"<select aria-label=\"fruit\"><option>apple</option><option>pear</option></select>"
	);
	assert_eq!(
		html_of("fruit = \"pear\"\nselect{ bind: fruit aria-label: \"fruit\"; [\"apple\", \"pear\"] }"),
		"<select value=\"pear\" data-warp-input=\"1\" aria-label=\"fruit\"><option>apple</option><option selected>pear</option></select>"
	);
}

#[test]
fn a_list_after_the_bound_variable_is_no_part_of_it() {
	assert_eq!(
		html_of("fruit = \"pear\"\nselect{ aria-label: \"fruit\" bind: fruit [\"apple\" \"pear\"] }"),
		"<select aria-label=\"fruit\" value=\"pear\" data-warp-input=\"1\"><option>apple</option><option selected>pear</option></select>"
	);
}

#[test]
fn a_map_gives_values_and_labels() {
	assert_eq!(
		html_of("size = \"l\"\nselect{ bind: size aria-label: \"size\"; [\"s\": \"small\", \"l\": \"large\"] }"),
		"<select value=\"l\" data-warp-input=\"1\" aria-label=\"size\"><option value=\"s\">small</option><option value=\"l\" selected>large</option></select>"
	);
}

#[test]
fn a_variable_list_gives_the_options() {
	assert_eq!(
		html_of("fruits = [\"apple\" \"pear\"]\nselect{ aria-label: \"fruit\"; fruits }"),
		"<select aria-label=\"fruit\"><option>apple</option><option>pear</option></select>"
	);
}
