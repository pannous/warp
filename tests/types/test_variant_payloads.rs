//! P179 (user, 2026-10-07): sum types work like optional. Payload fields by 1-based position `rgb(1,2,3)#1`, Rust's
//! `.0` `.1` as a fallback; a payload-free variant stays its symbol and `red is Color` is true; `a:int = Some(3)`
//! unwraps to 3; payload names `rgb(r, g, b)` name the fields. Positions work for any class instance (`p#2`).
use crate::common::fails_with;
use crate::is;

const COLOR: &str = "type Color = red | green | rgb(int, int, int)";
const OPTION: &str = "type Option[T] = Some(T) | None";

#[test]
fn payload_fields_by_position() {
	is!(&format!("{COLOR}; c = rgb(1, 2, 3); c#1"), 1);
	is!(&format!("{COLOR}; c = rgb(1, 2, 3); c#3"), 3);
	is!(&format!("{COLOR}; rgb(4, 5, 6)#2"), 5);
	is!(&format!("{OPTION}; s = Some(4); s#1 + s.value"), 8);
	is!("class P { x: int; y: int }; p = P(3, 4); p#2", 4);
	fails_with(&format!("{COLOR}; c = rgb(1, 2, 3); c#4"), "rgb has 3 fields");
}

#[test]
fn rust_style_positions_count_from_zero() {
	is!(&format!("{COLOR}; c = rgb(1, 2, 3); c.0"), 1);
	is!(&format!("{COLOR}; c = rgb(1, 2, 3); c.2"), 3);
}

#[test]
fn a_payload_free_variant_is_its_symbol_and_of_its_type() {
	is!(&format!("{COLOR}; red"), warp::symbol("red"));
	is!(&format!("{COLOR}; red is Color"), true);
	is!(&format!("{COLOR}; x = green; x is Color"), true);
	is!(&format!("{COLOR}; x = blue; x is Color"), false);
	is!(&format!("{COLOR}; rgb(1, 2, 3) is Color"), true);
}

#[test]
fn payload_names_name_the_fields() {
	is!("type Color = red | rgb(r, g, b); c = rgb(1, 2, 3); c.g", 2);
}

#[test]
fn a_typed_variable_unwraps_a_single_payload() {
	is!(&format!("{OPTION}; a: int = Some(3); a + 1"), 4);
}

#[test]
fn an_or_of_a_grouped_operation_keeps_its_right_side() {
	is!("class rgb{a:int}; x = green; ((x is rgb) or 0) or 5", 5);
	is!(&format!("{COLOR}; x = green; (x is Color) or true"), true);
}
