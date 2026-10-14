// card unit-possible (user: "x as unit possible synonym for quantity(x)"): `x as quantity` is quantity(x), the run-time
// quantity of a text or of a unit literal; `x as unit` is another word for it
use crate::is;

/// the text of the program's last value, a quantity's own `5km`
fn shown(code: &str) -> String {
	warp::wasm_emitter::eval(&format!("str({code})")).serialize().trim_matches('"').to_string()
}

#[test]
fn a_text_as_quantity_is_its_quantity() {
	assert_eq!(shown("\"5 km\" as quantity"), "5km");
	is!("x = \"2.5 km\"; (x as quantity).in(\"m\")", 2500);
}

#[test]
fn as_unit_is_as_quantity() {
	assert_eq!(shown("\"5 km\" as unit"), "5km");
	is!("(\"1 km\" as unit) > (\"999 m\" as unit)", true);
}
