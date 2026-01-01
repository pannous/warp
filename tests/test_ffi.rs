
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

use wasp::{eq, is, skip};
use wasp::wasp_parser::parse;

#[test] fn test_dynlib_import_emit() {
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
#[test] fn test_ffi_floor() {
    // Test: float64 . float64 (floor from libm);
    is!("import floor from 'm'\nfloor(3.7)", 3.0);
    is!("import floor from 'm'\nfloor(-2.3)", -3.0);
    is!("import floor from 'm'\nfloor(5.0)", 5.0);
}

#[test] fn test_ffi_strlen() {
    return; // clashes with wasp runtime strlen!
    // Test: char* . int32 (strlen from libc);
    is!("import strlen from \"c\"\nstrlen(\"hello\")", 5);
    is!("import strlen from \"c\"\nstrlen(\"\")", 0);
    // is!("import strlen from \"c\"\nstrlen(\"Wasp\")", 4);
}

#[test] fn test_ffi_atof() {
    let modul = loadNativeLibrary("c");
    assert!(modul);
    assert!(modul.functions.has("atof"));
    // double	 atof(const char *);
    assert!(modul.functions["atof"].signature.parameters.size() == 1);
    assert!(modul.functions["atof"].signature.parameters[0].typo == charp);
    assert!(modul.functions["atof"].signature.return_types.size() == 1);
    assert!(modul.functions["atof"].signature.return_types[0] == float64t);// not 32!!
    // Test: char* . float64 (atof from libc);
    is!("import atof from \"c\"\natof(\"3.14159\")", 3.14159);
    is!("import atof from \"c\"\natof(\"2.71828\")", 2.71828);
    is!("import atof from \"c\"\natof(\"42\")", 42.0);
}

#[test] fn test_ffi_fmin_wasp_file() {
    is!("test/wasp/ffi/test_ffi_fmin.wasp", 2.1);
}

#[test] fn test_ffi_fmin() {
    // Test: float64, float64 . float64 (fmin from libm);
    is!("import fmin from 'm'\nfmin(3.5, 2.1)", 2.1);
    is!("import fmin from 'm'\nfmin(100.0, 50.0)", 50.0);
    is!("import fmin from 'm'\nfmin(-5.0, -10.0)", -10.0);
}

#[test] fn test_ffi_combined() {
    // Combined tests using multiple FFI functions together

    // sqrt(abs(-16)) = sqrt(16) = 4.0
    is!("import sqrt from 'm'\nimport abs from \"c\"\nsqrt(abs(-16))", 4.0);

    // floor(fmin(3.7, 2.9)) = floor(2.9) = 2.0
    is!("import floor from 'm'\nimport fmin from 'm'\nfloor(fmin(3.7, 2.9))", 2.0);
}

// ============================================================================
// let Comparison Functions
// ============================================================================

#[test] fn test_ffi_strcmp() {
    let modul = loadNativeLibrary("c");
    assert!(modul);
    assert!(modul.functions.has("strcmp"));
    // int strcmp(const char* s1, const char* s2);
    assert!(modul.functions["strcmp"].signature.parameters.size() == 2);
    assert!(modul.functions["strcmp"].signature.parameters[0].typo == charp);
    assert!(modul.functions["strcmp"].signature.parameters[1].typo == charp);
    assert!(modul.functions["strcmp"].signature.return_types.size() == 1);
    assert!(modul.functions["strcmp"].signature.return_types[0] == int32t);
    // Test: int strcmp(const char* s1, const char* s2);
    is!("import strcmp from \"c\"\nstrcmp(\"hello\", \"hello\")", 0);
    is!("import strcmp from \"c\"\nx=strcmp(\"abc\", \"def\");x<0", 1);
    is!("import strcmp from \"c\"\nx=strcmp(\"xyz\", \"abc\");x>0", 1);
    is!("import strcmp from \"c\"\nstrcmp(\"\", \"\")", 0);
}

#[test] fn test_ffi_strncmp() {
    // Test: int strncmp(const char* s1, const char* s2, size_t n);
    is!("import strncmp from \"c\"\nstrncmp(\"hello\", \"help\", 3)", 0);
    is!("import strncmp from \"c\"\nx=strncmp(\"hello\", \"help\", 5);x!=0", 1);
}

// ============================================================================
// Additional Math Functions
// ============================================================================

#[test] fn test_ffi_ceil() {
    // Test: double ceil(double x);
    is!("import ceil from 'm'\nceil(3.2)", 4.0);
    is!("import ceil from 'm'\nceil(-2.8)", -2.0);
    is!("import ceil from 'm'\nceil(5.0)", 5.0);
    is!("import ceil from 'm'\nceil(0.1)", 1.0);
}

#[test] fn test_ffi_sin() {
    // Test: double sin(double x);
    is!("import sin from 'm'\nsin(0.0)", 0.0);
    is!("import sin from 'm'\nsin(1.5707963267948966)", 1.0);
    is!("import sin from 'm'\nimport abs from \"c\"\nabs(sin(3.141592653589793))<0.001", 1);
}

#[test] fn test_ffi_cos() {
    // Test: double cos(double x);
    is!("import cos from 'm'\ncos(0.0)", 1.0);
    is!("import cos from 'm'\nimport abs from \"c\"\nabs(cos(1.5707963267948966))<0.001", 1);
    is!("import cos from 'm'\ncos(3.141592653589793)", -1.0);
}

#[test] fn test_ffi_tan() {
    // Test: double tan(double x);
    is!("import tan from 'm'\ntan(0.0)", 0.0);
    is!("import tan from 'm'\ntan(0.7853981633974483)", 1.0);
}

#[test] fn test_ffi_fabs() {
    // Test: double fabs(double x);
    // Test: int32 . int32 (abs from libc);
    // Test: double . double (fabs from libc);
    // Test: float32 . float32 (fabsf from libc);
    is!("import fabs from 'm'\nfabs(3.14)", 3.14);
    is!("import fabs from 'm'\nfabs(-3.14)", 3.14);
    // is!("import fabs from 'm'\nfabs(0.0)", 0.0);
}

#[test] fn test_ffi_fmax() {
    // Test: double fmax(double x, double y);
    is!("import fmax from 'm'\nfmax(3.5, 2.1)", 3.5);
    is!("import fmax from 'm'\nfmax(100.0, 200.0)", 200.0);
    is!("import fmax from 'm'\nfmax(-5.0, -10.0)", -5.0);
}

#[test] fn test_ffi_fmod() {
    // Test: double fmod(double x, double y);
    is!("import fmod from 'm'\nfmod(5.5, 2.0)", 1.5);
    is!("import fmod from 'm'\nfmod(10.0, 3.0)", 1.0);
}

// ============================================================================
// let Conversion Functions
// ============================================================================

#[test] fn test_ffi_atoi() {
    // Test: int atoi(const char* str);
    is!("import atoi from \"c\"\natoi(\"42\")", 42);
    is!("import atoi from \"c\"\natoi(\"-123\")", -123);
    is!("import atoi from \"c\"\natoi(\"0\")", 0);
    is!("import atoi from \"c\"\natoi(\"999\")", 999);
}

#[test] fn test_ffi_atol() {
    // Test: long atol(const char* str);
    is!("import atol from \"c\"\natol(\"1234567\")", 1234567);
    is!("import atol from \"c\"\natol(\"-999999\")", -999999);
}

// ============================================================================
// Zero-Parameter Functions
// ============================================================================

#[test] fn test_ffi_rand() {
    // Test: int rand(void);
    is!("import rand from \"c\"\nx=rand();x>=0", 1);
    is!("import rand from \"c\"\nx=rand();y=rand();x!=y", 1);
}

// ============================================================================
// Combined/Complex Tests
// ============================================================================

#[test] fn test_ffi_trigonometry_combined() {
    // Test: sin²(x) + cos²(x) = 1 (Pythagorean identity);
    is!(r#"
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

#[test] fn test_ffi_string_math_combined() {
    // Test: Parse string numbers and do math
    is!(r#"import atoi from "c"
        x = atoi("10")
        y = atoi("20")
        x + y"#,
        30
    );

    is!("import atof from c;import ceil from 'm';ceil(atof('3.7'))",
        4.0
    );
}

#[test] fn test_ffi_string_comparison_logic() {
    // Test: Use strcmp for conditional logic
    is!(r#"import strcmp from "c"
result = strcmp("test", "test")
if result == 0 then 100 else 200"#,
        100
    );

    is!(r#"import strcmp from "c"
result = strcmp("aaa", "bbb")
if result < 0 then 1 else 0"#,
        1
    );
}

#[test] fn test_ffi_math_pipeline() {
    // Test: Chain multiple math functions
    is!(r#"import sin from 'm'
import floor from 'm'
import fabs from 'm'
fabs(floor(sin(3.14159)))"#,
        0.0
    );

    is!(r#"import ceil from 'm'
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

#[test] fn test_import_from_pattern_parse() {
    let code1 = "import abs from \"c\"";
    let parsed1 = parse(code1);
    eq!(parsed1.name, "import");

    let code2 = "import sqrt from \"m\"";
    let parsed2 = parse(code2);
}

#[test] fn test_import_from_pattern_emit() {
    skip!(

        is!("import abs from \"c\"\nabs(-42)", 42);
        is!("import floor from \"m\"\nimport ceil from \"m\"\nceil(floor(3.7))", 3.0);
        is!("import sqrt from \"m\"\nsqrt(16)", 4.0);
    );
}

#[test] fn test_import_from_vs_include() {
    let ffi_import = "import abs from \"c\"";
    let ffi_node = parse(ffi_import);
}

// ============================================================================
// C Header Parser Tests
// ============================================================================

#[test] fn test_extract_function_signature() {
    let c_code1 = "double sqrt(double x);";
    let parsed1 = parse(c_code1);
    let sig1;
    sig1.library = "m";
    extractFunctionSignature(c_code1, sig1);
    eq!(sig1.name, "sqrt");
    eq!(sig1.return_type, "double");
    eq!(sig1.param_types.size(), 1);
    eq!(sig1.param_types[0], "double");

    let c_code2 = "double fmin(double x, double y);";
    let parsed2 = parse(c_code2);
    let sig2;
    sig2.library = "m";
    extractFunctionSignature(c_code2, sig2);
    eq!(sig2.name, "fmin");
    eq!(sig2.return_type, "double");
    eq!(sig2.param_types.size(), 2);
    eq!(sig2.param_types[0], "double");
    eq!(sig2.param_types[1], "double");

    let c_code3 = "int strlen(char* str);";
    let parsed3 = parse(c_code3);
    let sig3;
    // extractFunctionSignature(c_code3, sig3);
    eq!(sig3.name, "strlen");
    eq!(sig3.return_type, "int");
    eq!(sig3.param_types.size(), 1);
    eq!(sig3.param_types[0], "char*");

}

// #[test] fn test_c_type_mapping() {
//     assert!(mapCTypeToWasp("double") == float64t);
//     assert!(mapCTypeToWasp("float") == float32t);
//     assert!(mapCTypeToWasp("int") == int32t);
//     assert!(mapCTypeToWasp("long") == i64);
//     assert!(mapCTypeToWasp("char*") == charp);
//     assert!(mapCTypeToWasp("const char*") == charp);
//     assert!(mapCTypeToWasp("void") == nils);
// }

// ============================================================================
// Main Test Runners
// ============================================================================

#[test] fn test_ffi_extended_emit() {
    test_ffi_strcmp();
    test_ffi_ceil();
    test_ffi_sin();
    test_ffi_cos();
    test_ffi_tan();
    test_ffi_fabs();
    test_ffi_fmax();
    test_ffi_fmod();
    test_ffi_atoi();
    test_ffi_rand();
    test_ffi_trigonometry_combined();
    test_ffi_string_math_combined();
    test_ffi_string_comparison_logic();
    test_ffi_math_pipeline();
    // test_ffi_signature_coverage();
}

#[test] fn test_ffi_import_pattern() {
    test_import_from_pattern_parse();
    test_import_from_pattern_emit();
    test_import_from_vs_include();
}

#[test] fn test_ffi_header_parser() {
    test_extract_function_signature();
    // test_c_type_mapping();
}

// ============================================================================
// SDL Graphics FFI Tests
// ============================================================================

#[test] fn test_ffi_sdl_init() {
    // Test: SDL_Init - Initialize SDL with timer subsystem (works headless);
    // SDL_INIT_TIMER = 0x00000001 (doesn't require display);
    // Returns 0 on success, non-zero on error
    is!("test/wasp/ffi/sdl/sdl_init.wasp", 0);

    // Test: SDL_Quit - Clean up SDL
    is!("import SDL_Quit from 'SDL2'\nSDL_Quit()\n42", 42);
}

#[test] fn test_ffi_sdl_window() {
    is!("test/wasp/ffi/sdl/sdl_init_quit.wasp", 1);
}

#[test] fn test_ffi_sdl_version() {
    // Test: SDL_GetVersion - Get SDL version info
    // This tests struct parameter passing via FFI
    is!("test/wasp/ffi/sdl/sdl_init_quit.wasp", 1);
}

#[test] fn test_ffi_sdl_combined() {
    // Combined test: Multiple SDL function imports
    // Tests that we can import multiple SDL functions in one program
    is!("test/wasp/ffi/sdl/sdl_get_ticks.wasp", 100);
}
#[test] fn test_ffi_sdl_debug() {
    // print results of SDL functions to debug FFI
    is!("test/wasp/ffi/sdl/sdl_debug.wasp", 1);
}

#[test] fn test_ffi_sdl_red_square_demo() {
    // DEMO: Display a red square using SDL2 via FFI
    // This will show an actual window with graphics
    is!("test/wasp/ffi/sdl/sdl_red_square_demo.wasp", 1);
}

#[test] fn test_ffi_sdl() {
    test_ffi_sdl_init();
    test_ffi_sdl_window();
    test_ffi_sdl_version();
    skip!(

    test_ffi_sdl_combined(); // broken after 48eb08f7817b28bb38eb1cc7756f938dc91116f1
        );
    // test_ffi_sdl_red_square_demo(); only live demo, not automated test
}

// ============================================================================
// Raylib Graphics FFI Tests
// ============================================================================
#[test] fn test_ffi_raylib_combined() {
    // Test: Multiple raylib imports in one program
    is!(r#"
import InitWindow from 'raylib'
import SetTargetFPS from 'raylib'
import CloseWindow from 'raylib'
InitWindow(800, 600, "Test")
SetTargetFPS(60)
CloseWindow()
100 "#,100);
}

#[test] fn test_ffi_raylib_simple_use_import() {
    let modul = loadNativeLibrary("raylib");
    assert!(modul);
    assert!(modul.functions.has("InitWindow"));
    assert!(modul.functions.has("DrawCircle"));
    assert!(modul.functions.has("WindowShouldClose"));
    assert!(modul.functions["InitWindow"].signature.parameters.size() == 3);
    assert!(modul.functions["DrawCircle"].signature.parameters.size() == 4);
    assert!(modul.functions["WindowShouldClose"].signature.parameters.size() == 0);
    assert!(modul.functions["WindowShouldClose"].signature.return_types.size() == 1);
    // eq!(modul.functions["WindowShouldClose"].signature.return_types[0],bools); // bool as int32
    // eq!(modul.functions["DrawCircle"].signature.parameters[3].typo,int32t);

    is!("samples/raylib_circle.wasp",0);
    // is!("samples/raylib_simple.wasp",0);

    // is!("samples/raylib_simple_use.wasp",0);
}

#[test] fn test_ffi_raylib() {
    let modul = loadNativeLibrary("raylib");
    assert!(modul);
    assert!(modul.functions.has("InitWindow"));
    assert!(modul.functions.has("DrawCircle"));
    assert!(modul.functions.has("BeginDrawing"));
    test_ffi_raylib_combined();
    skip!(

        test_ffi_raylib_simple_use_import(); // todo
        is!("test/wasp/ffi/raylib/raylib_init_close.wasp", 42);
    );
    // test_ffi_raylib_combined();
}

#[test] fn test_ffi_all() {
    // Main comprehensive test function that runs all FFI tests
    let modul = loadNativeLibrary("m");
    // assert!(modul);
    // assert!(modul.functions.has("fmin"));
    test_ffi_atof(); // careful this is already a built-in wasp library function
    test_ffi_strcmp();
    test_ffi_fmin();
    test_ffi_fmin_wasp_file();
    test_ffi_fabs(); // careful this is already a built-in wasm operator
    test_ffi_floor(); // careful this is already a built-in wasm operator
    test_ffi_strlen(); // careful this is already a built-in wasp library function
    test_ffi_combined();
    // test_ffi_signature_detection();
    test_ffi_header_parser();
    test_ffi_sdl();
    // test_ffi_raylib();
    // test_dynlib_import_emit();
}

fn loadNativeLibrary(p0: &str) -> _ {
    todo!()
}
