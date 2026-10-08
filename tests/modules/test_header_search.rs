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

#[test]
fn the_sdl_headers_declare_the_window_functions() {
	// samples/sdl_red_square.wasp: SDL_CreateWindow and SDL_DestroyWindow are declared in SDL_video.h
	for name in ["SDL_CreateWindow", "SDL_DestroyWindow", "SDL_CreateRenderer", "SDL_Delay"] {
		assert!(warp::ffi::get_ffi_signature_from_lib(name, "SDL2").is_some(), "{name}");
	}
}

#[test]
fn an_import_nothing_declares_says_why() {
	crate::common::fails_with("import foo from 'nolib'; foo(1)", "foo is imported from nolib, but no header of nolib is found");
	crate::common::fails_with("import SDL_Nothing from 'SDL2'; SDL_Nothing(1)", "SDL_Nothing is imported from SDL2, but its headers do not declare it");
}
