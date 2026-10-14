// card cleanup-ffi-f32: one call per C signature the FFI linker once had a typed wrapper for (ffi/link.rs), so the
// generic wrapper taking them over is checked: void, int, bool, float, double results, int, float, double parameters.
// The raylib calls that need a window (InitWindow, DrawCircle, DrawRectangle, DrawText) are left out
use crate::is;
use warp::formats::ffi_parser::find_library_headers;

#[test]
fn libc_signatures() {
	is!("import sranddev from 'c'\nsranddev()\n1", 1); // _V
	is!("import srand from 'c'\nimport rand from 'c'\nsrand(7)\na=rand()\nsrand(7)\na==rand()", true); // I_V, _I
	is!("import abs from 'c'\nabs(-5)", 5); // I_I
	is!("import sqrtf from 'c'\nsqrtf(2.25)", 1.5); // F_F
	is!("import fabsf from 'c'\nfabsf(-2.5)", 2.5); // F_F
	is!("import sqrt from 'c'\nsqrt(2.25)", 1.5); // D_D
	is!("import pow from 'c'\npow(2.0, 10.0)", 1024.0); // DD_D
	is!("import ldexpf from 'c'\nldexpf(1.5, 2)", 6.0); // FI_F
}

#[test]
fn raylib_signatures_without_a_window() {
	if find_library_headers("raylib").is_empty() {
		eprintln!("raylib: NOT INSTALLED - skipping");
		return;
	}
	is!("import IsWindowReady from 'raylib'\nIsWindowReady()", 0); // _B
	is!("import GetMouseX from 'raylib'\nGetMouseX()", 0); // _I
	is!("import GetFrameTime from 'raylib'\nGetFrameTime()", 0.0); // _F
	is!("import GetTime from 'raylib'\nGetTime()", 0.0); // _D, 0 before InitWindow
	is!("import SetTraceLogLevel from 'raylib'\nSetTraceLogLevel(7)\n1", 1); // I_V
	is!("import IsKeyPressed from 'raylib'\nIsKeyPressed(65)", 0); // I_B
	is!("import SetMouseOffset from 'raylib'\nSetMouseOffset(1, 2)\n1", 1); // II_V
	is!("import GetRandomValue from 'raylib'\nGetRandomValue(4, 4)", 4); // II_I
	is!("import TextToInteger from 'raylib'\nTextToInteger('123')", 123); // P_I
}
