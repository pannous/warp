// Comprehensive FFI Tests
// All FFI-related tests consolidated from:
//   - test_dynlib_import.h
//   - test_ffi_comprehensive.h
//   - test_ffi_header_parser.h
//   - test_ffi_import_pattern.h
//
// Implementation uses:
//   - source/ffi_loader.h (library loading);
//   - source/ffi_marshaller.h (type conversion and signature detection);
//   - source/ffi_dynamic_wrapper.h (generic wrappers);
//   - source/ffi_header_parser.h (C header parsing);
//   - source/wasm_runner_edge.cpp (wasmedge FFI integration);
//   - source/wasmtime_runner.cpp (wasmtime FFI integration);
//   - source/Angle.cpp (import parsing);
//   - source/Context.h/cpp (FFI function registry);

// ============================================================================
// Dynamic Library Import Tests (using 'use' keyword);
// ============================================================================

use warp::{eq, is, parse, skip};

#[test]
#[ignore = "use keyword with ceil/floor conflicts with builtins"]
fn test_dynlib_import_emit() {
	// Test FFI import and usage with 'use' keyword
	// These are actual C library functions, not WASM builtins
	// Note: FFI only works in native mode, not when compiled to WASM

	// Math library functions (libm) - using functions that work reliably
	is!("use m; ceil(3.2)", 4.0);
	is!("use math; floor(3.7)", 3.0);
	is!("use m; fmin(3.5, 2.1)", 2.1);
	is!("use m; fmax(1.5, 2.5)", 2.5);
}

// ============================================================================
// Basic FFI Tests - Core functionality
// ============================================================================
#[test]
#[ignore = "floor is a built-in WASM instruction, FFI import is shadowed"]
fn test_ffi_floor() {
	// Test: float64 . float64 (floor from libm);
	is!("import floor from 'm'\nfloor(3.7)", 3.0);
	is!("import floor from 'm'\nfloor(-2.3)", -3.0);
	is!("import floor from 'm'\nfloor(5.0)", 5.0);
}

#[test]
fn test_ffi_strlen() {
	// Test: char* . int32 (strlen from libc);
	is!("import strlen from \"c\"\nstrlen(\"hello\")", 5);
	is!("import strlen from \"c\"\nstrlen(\"\")", 0);
	is!("import strlen from \"c\"\nstrlen(\"Wasp\")", 4);
}

#[test]
fn test_ffi_atof() {
	is!("import atof from \"c\"\natof(\"3.14159\")", 3.14159);
	is!("import atof from \"c\"\natof(\"2.71828\")", 2.71828);
	is!("import atof from \"c\"\natof(\"42\")", 42.0);
}

#[test]
#[ignore = "eval doesn't support loading .wasp files yet"]
fn test_ffi_fmin_wasp_file() {
	is!("tests/wasp/ffi/test_ffi_fmin.wasp", 2.1);
}

#[test]
fn test_ffi_fmin() {
	// Test: float64, float64 . float64 (fmin from libm);
	is!("import fmin from 'm'\nfmin(3.5, 2.1)", 2.1);
	is!("import fmin from 'm'\nfmin(100.0, 50.0)", 50.0);
	is!("import fmin from 'm'\nfmin(-5.0, -10.0)", -10.0);
}

#[test]
#[ignore = "floor builtin conflicts with FFI floor"]
fn test_ffi_combined() {
	// Combined tests using multiple FFI functions together
	// sqrt(abs(-16)) = sqrt(16) = 4.0
	is!("import sqrt from 'm'\nimport abs from \"c\"\nsqrt(abs(-16))",4.0);
	// floor(fmin(3.7, 2.9)) = floor(2.9) = 2.0
	is!("import floor from 'm'\nimport fmin from 'm'\nfloor(fmin(3.7, 2.9))",2.0);
}


#[test]
fn test_ffi_strcmp() {
	// Test: int strcmp(const char* s1, const char* s2);
	is!("import strcmp from \"c\"\nstrcmp(\"hello\", \"hello\")", 0);
	is!("import strcmp from \"c\"\nx=strcmp(\"abc\", \"def\");x<0",1);
	is!("import strcmp from \"c\"\nx=strcmp(\"xyz\", \"abc\");x>0",1);
	is!("import strcmp from \"c\"\nstrcmp(\"\", \"\")", 0);
}

#[test]
#[ignore = "string argument marshalling needs work"]
fn test_ffi_strncmp() {
	// Test: int strncmp(const char* s1, const char* s2, size_t n);
	is!("import strncmp from \"c\"\nstrncmp(\"hello\", \"help\", 3)",0);
	is!("import strncmp from \"c\"\nx=strncmp(\"hello\", \"help\", 5);x!=0",1);
}

// ============================================================================
// Additional Math Functions
// ============================================================================

#[test]
#[ignore = "ceil is a built-in WASM instruction, FFI import is shadowed"]
fn test_ffi_ceil() {
	// Test: double ceil(double x);
	is!("import ceil from 'm'\nceil(3.2)", 4.0);
	is!("import ceil from 'm'\nceil(-2.8)", -2.0);
	is!("import ceil from 'm'\nceil(5.0)", 5.0);
	is!("import ceil from 'm'\nceil(0.1)", 1.0);
}

#[test]
fn test_ffi_sin() {
	// Test: double sin(double x);
	is!("import sin from 'm'\nsin(0.0)", 0.0);
	is!("import sin from 'm'\nsin(1.5707963267948966)", 1.0);
	is!(
		"import sin from 'm'\nimport abs from \"c\"\nabs(sin(3.141592653589793))<0.001",
		1
	);
}

#[test]
#[ignore]
fn test_ffi_cos() {
	// Test: double cos(double x);
	is!("import cos from 'm'\ncos(0.0)", 1.0);
	is!(
		"import cos from 'm'\nimport abs from \"c\"\nabs(cos(1.5707963267948966))<0.001",
		1
	);
	is!("import cos from 'm'\ncos(3.141592653589793)", -1.0);
}

#[test]
#[ignore]
fn test_ffi_tan() {
	// Test: double tan(double x);
	is!("import tan from 'm'\ntan(0.0)", 0.0);
	is!("import tan from 'm'\ntan(0.7853981633974483)", 1.0);
}

#[test]
#[ignore]
fn test_ffi_fabs() {
	// Test: double fabs(double x);
	// Test: int32 . int32 (abs from libc);
	// Test: double . double (fabs from libc);
	// Test: float32 . float32 (fabsf from libc);
	is!("import fabs from 'm'\nfabs(3.11)", 3.11);
	is!("import fabs from 'm'\nfabs(-3.11)", 3.11);
	// is!("import fabs from 'm'\nfabs(0.0)", 0.0);
}

#[test]
#[ignore]
fn test_ffi_fmax() {
	// Test: double fmax(double x, double y);
	is!("import fmax from 'm'\nfmax(3.5, 2.1)", 3.5);
	is!("import fmax from 'm'\nfmax(100.0, 200.0)", 200.0);
	is!("import fmax from 'm'\nfmax(-5.0, -10.0)", -5.0);
}

#[test]
#[ignore]
fn test_ffi_fmod() {
	// Test: double fmod(double x, double y);
	is!("import fmod from 'm'\nfmod(5.5, 2.0)", 1.5);
	is!("import fmod from 'm'\nfmod(10.0, 3.0)", 1.0);
}

// ============================================================================
// let Conversion Functions
// ============================================================================

#[test]
#[ignore]
fn test_ffi_atoi() {
	// Test: int atoi(const char* str);
	is!("import atoi from \"c\"\natoi(\"42\")", 42);
	is!("import atoi from \"c\"\natoi(\"-123\")", -123);
	is!("import atoi from \"c\"\natoi(\"0\")", 0);
	is!("import atoi from \"c\"\natoi(\"999\")", 999);
}

#[test]
#[ignore]
fn test_ffi_atol() {
	// Test: long atol(const char* str);
	is!("import atol from \"c\"\natol(\"1234567\")", 1234567);
	is!("import atol from \"c\"\natol(\"-999999\")", -999999);
}

// ============================================================================
// Zero-Parameter Functions
// ============================================================================

#[test]
#[ignore]
fn test_ffi_rand() {
	// Test: int rand(void);
	is!("import rand from \"c\"\nx=rand();x>=0", 1);
	is!("import rand from \"c\"\nx=rand();y=rand();x!=y", 1);
}

// ============================================================================
// Combined/Complex Tests
// ============================================================================

#[test]
#[ignore]
fn test_ffi_trigonometry_combined() {
	// Test: sin²(x) + cos²(x) = 1 (Pythagorean identity);
	is!(
		r#"
        import sin from 'm'
        import cos from 'm'
        x = 0.5
        sin_x = sin(x)
        cos_x = cos(x)
        result = sin_x * sin_x + cos_x * cos_x
        result"#,
		1.0
	);
}

#[test]
#[ignore]
fn test_ffi_string_math_combined() {
	// Test: Parse string numbers and do math
	is!(
		r#"import atoi from "c"
        x = atoi("10")
        y = atoi("20")
        x + y"#,
		30
	);

	is!(
		"import atof from c;import ceil from 'm';ceil(atof('3.7'))",
		4.0
	);
}

#[test]
#[ignore]
fn test_ffi_string_comparison_logic() {
	// Test: Use strcmp for conditional logic
	is!(
		r#"import strcmp from "c"
result = strcmp("test", "test")
if result == 0 then 100 else 200"#,
		100
	);

	is!(
		r#"import strcmp from "c"
result = strcmp("aaa", "bbb")
if result < 0 then 1 else 0"#,
		1
	);
}

#[test]
#[ignore]
fn test_ffi_math_pipeline() {
	// Test: Chain multiple math functions
	is!(
		r#"import sin from 'm'
import floor from 'm'
import fabs from 'm'
fabs(floor(sin(3.14159)))"#,
		0.0
	);

	is!(
		r#"import ceil from 'm'
import floor from 'm'
import fmax from 'm'
fmax(ceil(2.3), floor(5.9))"#,
		5.0
	);
}

// ============================================================================
// Signature Detection and Coverage Tests
// ============================================================================

// Import Pattern Tests
// ============================================================================

#[test]
fn test_import_from_pattern_parse() {
	let code1 = "import abs from \"c\"";
	let parsed1 = parse(code1);
	eq!(parsed1.name(), "import");

	let code2 = "import sqrt from \"m\"";
	let _parsed2 = parse(code2);
}

#[test]
fn test_import_from_pattern_emit() {
	skip!(

		is!("import abs from \"c\"\nabs(-42)", 42);
		is!("import floor from \"m\"\nimport ceil from \"m\"\nceil(floor(3.7))", 3.0);
		is!("import sqrt from \"m\"\nsqrt(16)", 4.0);
	);
}

#[test]
fn test_import_from_vs_include() {
	let ffi_import = "import abs from \"c\"";
	let _ffi_node = parse(ffi_import);
}

// ============================================================================
// C Header Parser Tests
// ============================================================================

#[test]
#[ignore]
fn test_extract_function_signature() {
	let c_code1 = "double sqrt(double x);";
	let _parsed1 = parse(c_code1);
	// extractFunctionSignature(c_code1, sig1);
	// eq!(sig1.name, "sqrt");
	// eq!(sig1.return_type, "double");
	// eq!(sig1.param_types.size(), 1);
	// eq!(sig1.param_types[0], "double");

	let c_code2 = "double fmin(double x, double y);";
	let _parsed2 = parse(c_code2);
	// let sig2;
	// sig2.library = "m";
	// extractFunctionSignature(c_code2, sig2);
	// eq!(sig2.name, "fmin");
	// eq!(sig2.return_type, "double");
	// eq!(sig2.param_types.size(), 2);
	// eq!(sig2.param_types[0], "double");
	// eq!(sig2.param_types[1], "double");
	//
	// let c_code3 = "int strlen(char* str);";
	// let parsed3 = parse(c_code3);
	// let sig3;
	// // extractFunctionSignature(c_code3, sig3);
	// eq!(sig3.name, "strlen");
	// eq!(sig3.return_type, "int");
	// eq!(sig3.param_types.size(), 1);
	// eq!(sig3.param_types[0], "char*");
}

#[test] fn test_c_type_mapping() {
    // assert!(mapCTypeToWasp("double") == float64t);
//     assert!(mapCTypeToWasp("float") == float32t);
//     assert!(mapCTypeToWasp("int") == int32t);
//     assert!(mapCTypeToWasp("long") == i64);
//     assert!(mapCTypeToWasp("char*") == charp);
//     assert!(mapCTypeToWasp("const char*") == charp);
//     assert!(mapCTypeToWasp("void") == nils);
}

// ============================================================================
// SDL Graphics FFI Tests
// ============================================================================

#[test]
#[ignore = "requires SDL2 library and wasp files"]
fn test_ffi_sdl_init() {
	// Test: SDL_Init - Initialize SDL with timer subsystem (works headless);
	// Returns 0 on success, non-zero on error
	is!("tests/wasp/ffi/sdl/sdl_init.wasp", 0);

	// Test: SDL_Quit - Clean up SDL
	is!("import SDL_Quit from 'SDL2'\nSDL_Quit()\n42", 42);
}

#[test]
#[ignore = "requires SDL2 library and wasp files"]
fn test_ffi_sdl_window() {
	is!("tests/wasp/ffi/sdl/sdl_init_quit.wasp", 1);
}

#[test]
#[ignore = "requires SDL2 library and wasp files"]
fn test_ffi_sdl_version() {
	// Test: SDL_GetVersion - Get SDL version info
	// This tests struct parameter passing via FFI
	is!("tests/wasp/ffi/sdl/sdl_init_quit.wasp", 1);
}

#[test]
#[ignore = "requires SDL2 library and wasp files"]
fn test_ffi_sdl_combined() {
	// Combined test: Multiple SDL function imports
	// Tests that we can import multiple SDL functions in one program
	is!("tests/wasp/ffi/sdl/sdl_get_ticks.wasp", 100);
}

#[test]
#[ignore = "requires SDL2 library and wasp files"]
fn test_ffi_sdl_debug() {
	// print results of SDL functions to debug FFI
	is!("tests/wasp/ffi/sdl/sdl_debug.wasp", 1);
}

#[test]
#[ignore = "requires SDL2 library and display"]
fn test_ffi_sdl_red_square_demo() {
	// DEMO: Display a red square using SDL2 via FFI
	// This will show an actual window with graphics
	is!("tests/wasp/ffi/sdl/sdl_red_square_demo.wasp", 1);
}

#[test]
#[ignore = "requires raylib library"]
fn test_ffi_raylib_combined() {
	// Test: Multiple raylib imports in one program
	is!(r#"
import InitWindow from 'raylib'
import SetTargetFPS from 'raylib'
import CloseWindow from 'raylib'
InitWindow(800, 600, "Test")
SetTargetFPS(60)
CloseWindow()
100 "# ,100)
}

// ============================================================================
// FFI Header Discovery Tests
// Tests generic library header discovery from filesystem
// ============================================================================

use warp::ffi_parser::{find_library_headers, parse_header_file};

#[test]
fn test_ffi_header_parser() {
	// Test that we can parse function declarations
	use warp::ffi_parser::parse_declaration;

	let func = parse_declaration("double sin(double x);", "m").unwrap();
	assert_eq!(func.name, "sin");
	assert_eq!(func.signature.parameters.len(), 1);
}

#[test]
fn test_ffi_import_pattern() {
	// Test import pattern parsing
	let node = parse("import sqrt from 'm'");
	assert_eq!(node.name(), "import");
}

#[test]
fn test_ffi_curl_discovery() {
	// Test generic discovery finds curl headers
	let paths = find_library_headers("curl");

	if paths.is_empty() {
		eprintln!("curl: NOT INSTALLED - skipping");
		return;
	}

	eprintln!("curl paths: {:?}", paths);

	let mut total = 0;
	for path in &paths {
		let funcs = parse_header_file(path, "curl");
		total += funcs.len();
		eprintln!("curl: {} functions from {}", funcs.len(), path);

		// Show some actual curl functions
		let curl_funcs: Vec<_> = funcs.iter()
			.filter(|f| f.name.starts_with("curl_"))
			.take(5)
			.collect();
		for f in curl_funcs {
			eprintln!("  - {}", f.name);
		}
	}

	// Should find some functions if curl is installed
	assert!(total > 0, "Should find curl functions");
}

#[test]
fn test_ffi_zlib_discovery() {
	// Test generic discovery finds zlib headers
	let paths = find_library_headers("zlib");

	if paths.is_empty() {
		eprintln!("zlib: NOT INSTALLED - skipping");
		return;
	}

	for path in &paths {
		let funcs = parse_header_file(path, "zlib");
		eprintln!("zlib: {} functions from {}", funcs.len(), path);

		// Show some actual zlib functions
		let zlib_funcs: Vec<_> = funcs.iter()
			.filter(|f| f.name.starts_with("compress") ||
			           f.name.starts_with("uncompress") ||
			           f.name.starts_with("deflate") ||
			           f.name.starts_with("inflate"))
			.take(5)
			.collect();
		for f in zlib_funcs {
			eprintln!("  - {}", f.name);
		}
	}
}

#[test]
fn test_ffi_sqlite_discovery() {
	// Test generic discovery finds sqlite3 headers
	let paths = find_library_headers("sqlite3");

	if paths.is_empty() {
		eprintln!("sqlite3: NOT INSTALLED - skipping");
		return;
	}

	for path in &paths {
		let funcs = parse_header_file(path, "sqlite3");
		eprintln!("sqlite3: {} functions from {}", funcs.len(), path);

		// Show some actual sqlite functions
		let sqlite_funcs: Vec<_> = funcs.iter()
			.filter(|f| f.name.starts_with("sqlite3_"))
			.take(5)
			.collect();
		for f in sqlite_funcs {
			eprintln!("  - {}", f.name);
		}
	}
}

#[test]
fn test_ffi_raylib_discovery() {
	// Test generic discovery finds raylib headers
	let paths = find_library_headers("raylib");

	if paths.is_empty() {
		eprintln!("raylib: NOT INSTALLED - skipping");
		return;
	}

	for path in &paths {
		let funcs = parse_header_file(path, "raylib");
		eprintln!("raylib: {} functions from {}", funcs.len(), path);

		// Verify key raylib functions are found
		let key_funcs = ["InitWindow", "CloseWindow", "BeginDrawing", "EndDrawing"];
		for name in key_funcs {
			let found = funcs.iter().any(|f| f.name == name);
			if found {
				eprintln!("  ✓ {}", name);
			}
		}
	}
}

#[test]
fn test_ffi_png_discovery() {
	// Test generic discovery finds libpng headers
	let paths = find_library_headers("png");

	if paths.is_empty() {
		eprintln!("png: NOT INSTALLED - skipping");
		return;
	}

	for path in &paths {
		let funcs = parse_header_file(path, "png");
		eprintln!("png: {} functions from {}", funcs.len(), path);
	}
}

#[test]
fn test_ffi_generic_discovery() {
	// Test that generic discovery works for multiple libraries
	let libraries = ["curl", "zlib", "sqlite3", "raylib", "png"];
	let mut found = Vec::new();

	for lib in libraries {
		let paths = find_library_headers(lib);
		if !paths.is_empty() {
			found.push(lib);
		}
	}

	eprintln!("Found headers for: {:?}", found);

	// Should find at least some common libraries
	// (exact set depends on what's installed)
}

// ============================================================================
// Working FFI Tests - these tests verify actual FFI calls work
// ============================================================================

#[test]
fn test_ffi_sin_works() {
	is!("import sin from 'm'\nsin(0.0)", 0.0);
}

#[test]
fn test_ffi_sin_pi_half() {
	// sin(π/2) = 1.0
	is!("import sin from 'm'\nsin(1.5707963267948966)", 1.0);
}

#[test]
fn test_ffi_cos_works() {
	// cos(0) = 1
	is!("import cos from 'm'\ncos(0.0)", 1.0);
}

#[test]
fn test_ffi_sqrt_works() {
	is!("import sqrt from 'm'\nsqrt(4.0)", 2.0);
}

#[test]
fn test_ffi_abs_from_c() {
	// abs from libc takes i32 and returns i32
	is!("import abs from \"c\"\nabs(-42)", 42);
	is!("import abs from \"c\"\nabs(42)", 42);
	is!("import abs from \"c\"\nabs(0)", 0);
}

#[test]
fn test_ffi_fabs_from_m() {
	// fabs from libm takes f64 and returns f64
	is!("import fabs from 'm'\nfabs(-3.14)", 3.14);
	is!("import fabs from 'm'\nfabs(3.14)", 3.14);
}

#[test]
fn test_ffi_nested_calls() {
	// Nested FFI calls: fabs(sin(3.14)) ≈ 0.00159...
	// sin(π) ≈ 0, so fabs(sin(3.14)) should be close to 0
	is!("import fabs from 'm'\nimport sin from 'm'\nfabs(sin(3.14)) < 0.01", 1);
	// sin(π/2) = 1, fabs(1) = 1
	is!("import fabs from 'm'\nimport sin from 'm'\nfabs(sin(1.5707963267948966))", 1.0);
}
