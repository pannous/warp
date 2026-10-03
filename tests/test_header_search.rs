// The C headers FFI signatures come from: a header is looked up by name in one ordered list of include directories
// (WARP_INCLUDE overrides the list; the browser test runner points it at the one folder it serves) and the first
// directory holding it wins; only SDL_ names make the implicit lookup read the SDL headers
use warp::ffi::implicit_header_libraries;
use warp::ffi_parser::find_header_in;

#[test]
fn a_header_comes_from_the_first_include_directory_holding_it() {
	let directories = ["tests/fixtures/no_such_directory", "tests/fixtures/include", "tests/fixtures/include_shadowed"];
	assert_eq!(find_header_in("warp_probe.h", &directories), Some("tests/fixtures/include/warp_probe.h".to_string()));
	assert_eq!(find_header_in("no_such_header.h", &directories), None);
}

#[test]
fn only_sdl_names_read_the_sdl_headers() {
	assert!(implicit_header_libraries("SDL_Init").contains(&"SDL2"));
	assert!(!implicit_header_libraries("sqrt").contains(&"SDL2"));
	assert!(implicit_header_libraries("strlen").contains(&"c"));
}
