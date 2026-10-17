// card raylib-float: the float result of a function imported from a library beyond libc and libm is a float in
// arithmetic and comparisons, as it is for libm's (`GetTime() >= 0.0` failed: "GetTime is a float where an exact Int
// is expected")
use crate::is;
use warp::formats::ffi_parser::find_library_headers;

#[test]
fn a_raylib_float_result_is_a_float() {
	if find_library_headers("raylib").is_empty() {
		eprintln!("raylib: NOT INSTALLED - skipping");
		return;
	}
	is!("import GetTime from 'raylib'\nGetTime() >= 0.0", true); // double, 0 before InitWindow
	is!("import GetFrameTime from 'raylib'\nGetFrameTime() + 1.5", 1.5); // float
	is!("import GetTime from 'raylib'\nt = GetTime()\nt * 2.0", 0.0);
}
