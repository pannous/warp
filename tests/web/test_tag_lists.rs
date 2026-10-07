//! card ul-li: among an element's children a tag word applied to a list makes one tag per item, as broadcasting does
//! for functions: `ul{ li all fruits }` and `ul{ li ["apple" "pear"] }` are `ul{ [li{ fruit } for fruit in fruits] }`
//! (they rendered "<ul>liallapplepear</ul>")
use warp::markup::{is_markup, to_html};
use warp::wasm_emitter::eval;

const ITEMS: &str = "<ul><li>apple</li><li>pear</li></ul>";

fn html_of(code: &str) -> String {
	let value = eval(code);
	assert!(is_markup(&value), "no markup: {}", value.serialize());
	to_html(&value)
}

#[test]
fn a_tag_over_all_items_of_a_list() {
	assert_eq!(html_of("fruits = [\"apple\" \"pear\"]\nul{ li all fruits }"), ITEMS);
}

#[test]
fn a_tag_over_a_list_literal() {
	assert_eq!(html_of("ul{ li [\"apple\" \"pear\"] }"), ITEMS);
	assert_eq!(html_of("ul{ h2{ \"Fruit\" } li all [\"apple\" \"pear\"] }"), "<ul><h2>Fruit</h2><li>apple</li><li>pear</li></ul>");
}

#[test]
fn a_tag_of_one_value_stays_one_tag() {
	assert_eq!(html_of("ul{ li \"apple\" }"), "<ul><li>apple</li></ul>");
}
