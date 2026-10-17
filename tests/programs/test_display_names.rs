// decision display-names: display.density is the pixels per length in dpi (other names display.dpi, display.ppi),
// display.size width × height in px (display.dimensions), display.diagonal a length; display.resolution is ambiguous and
// reads the size with a hint asking which was meant
use crate::is;
use warp::normalize::capture_hints;
use warp::wasm_emitter::eval;

#[test]
fn the_display_has_a_density_a_size_and_a_diagonal() {
	is!("display.density == display.dpi and display.ppi == display.dpi", true);
	is!("display.size == [display.width, display.height] px", true);
	is!("display.diagonal > 1 inch and display.diagonal < 1000 inch", true);
	is!("display.density = 300 dpi\n1inch * display.density == 300px", true);
}

#[test]
fn other_names_of_a_display_property_name_it() {
	let (_, hints) = capture_hints(|| eval("display.resolution"));
	let hint = hints.iter().find(|hint| hint.canonical == "display.size").expect("a hint naming display.size");
	assert!(hint.reason.contains("density (dpi) or size (px)?"), "{}", hint.reason);
	let (_, hints) = capture_hints(|| eval("display.dimensions"));
	assert!(hints.iter().any(|hint| hint.canonical == "display.size"), "{hints:?}");
}
