# FFI end to end: pure pipeline + warp-host (Warp Wrapper, 2026-10-22/23)

Branch **pure-c-imports** (worktree /workspace/cow/trees/pure-c-imports, based on warp-host-run-wasm fcd5fd13a).
[M] measured on the box (Linux x86-64), [I] inferred. Times are CEST (Europe/Berlin).

## 1. Inventory: what declares C functions

| file(s) | library | functions | type shapes | tests | class |
|---|---|---|---|---|---|
| lib/math.warp:77-103 | m (libm) | 27 foreign lines | f64 → f64 | test_libm_*, test_ffi | (a) pure lines |
| lib/c.warp | c, m | was 8 libc + 3 libm + 8 sqlite3 lines; now ~40 libc/libm lines | i32, i64, f64, f32, text, handle, out | tests/host/test_ffi.sh | (a) |
| `use sqlite3` (tests/ffi/test_ffi_handles, test_ffi_text_results, test_ffi_gaps; 10 uses) | sqlite3 | Rust: every function of sqlite3.h | handles, out-pointers (`sqlite3 **`), text in/out | as listed | (b) on Rust (src/ffi/header.rs parses sqlite3.h at compile time); **now (a)**: lib/native/sqlite3.warp |
| `use z`/`use zlib`/`import compressBound from "z"` (test_ffi_gaps, test_ffi_text_results) | z | compressBound, adler32, zlibVersion | i64, bytes (`const Bytef *` + length) | 5 lines | (b) on Rust (headers, Bytes/Length params); now (a) lib/native/z.warp |
| `use c`, `import f from "c"` (tests/ffi/test_ffi*.warp, test_libc_results; 43+11 imports, 13 uses) | c | abs atof atoi atol fabsf getenv ldexpf pow rand sqrt sqrtf srand sranddev strcmp strlen strncmp strtod strtol tolower toupper, fopen/fclose | i32, i64, f64, f32, text, out (strtod endptr), FILE* handle | test_ffi (66 is!), test_ffi.pairs (15), test_libc_results (12) | (b) on Rust: headers + HAND_LINKED_LIBC shims (c_calls.rs link_libc_functions: strcmp/strncmp take (ptr,len) pairs); pure: (a) lib/c.warp |
| `import f from 'm'` (50 + 8) | m | cbrt ceil cos fabs floor fmax fmin fmod hypot ldexp sin sqrt tan | f64 | test_ffi*, test_libm_* | (a) |
| samples/graphics/raylib_*.warp, tests/warp/ffi/raylib/*.warp (46 imports) | raylib | InitWindow CloseWindow BeginDrawing EndDrawing ClearBackground DrawCircle DrawRectangle DrawText GetFrameTime GetMouseX GetRandomValue GetTime IsKeyPressed IsWindowReady SetMouseOffset SetTargetFPS SetTraceLogLevel TextToInteger | i32, f32 (radius, frame time), text, Color struct by value (as i32), bool | test_ffi (raylib, Mac only) | (b) on Rust: header parse, struct_layouts.rs, NOT_THREAD_SAFE mutex, window refusal (src/ffi/link.rs:8); now (a) lib/native/raylib.warp |
| samples/graphics/sdl_red_square.warp, tests/warp/ffi/sdl/*.warp (17) | SDL2 | SDL_Init Quit GetTicks Delay CreateWindow CreateRenderer SetRenderDrawColor RenderClear RenderPresent Destroy* | i32, text, handles | test_ffi_sdl_* (Mac) | (b) headers; now (a) lib/native/SDL2.warp |
| lib/wasmtime.warp (`use wasmtime`, tests/ffi/test_wasmtime_ffi.warp) | wasmtime | ~15 | structs in/out as maps (wasmtime_val_t, instance), error out-pointers, `.of.func` | 3 | **(b)**: c_calls.rs StructIn/StructsIn/StructOut/StructsOut/ErrorOut + c_structs.rs + header struct layouts |
| lib/hash.warp, json.warp, regex.warp | none (Rust crates) | std_pure("hash"/"json"/"regex", …) | | | (b) Rust bridge: src/modules/std_adapters.rs (sha2, crc32, serde_json, regex crates) |
| lib/net.warp, os.warp, file.warp, shell.warp, sound.warp | none | std_io(…) | | | (b) Rust bridge std_adapters.rs (warp-host words.rs re-implements os/file) |

## 2. Old Rust pipeline (src/, read-only)
- Imports: module = the library as written (`c`, `m`, `sqlite3`, `z`), field = the C symbol (src/wasm_emitter/import_manager.rs:85).
- Types from the **headers** at compile time (src/ffi/header.rs); runtime re-reads headers (src/ffi/link.rs).
- Texts: an i32 address of a NUL-terminated copy in linear memory; text results a ref built via the exported new_text/text_heap.
- Custom section **`warp.c_calls`** (not warp.c_functions): `module\tname\tletters\tresult`, letters t/l/s/n/o/b/k (src/modules/wasm_modules/c_calls.rs). Only for libc and module paths.
- Executables carry `WRPCcfun` + CFunction lines (c_signatures.rs).

Old compiler → .wasm → warp-host (the warp-host-run-wasm build) [M]:

| program | `warp run` (Rust) | Rust .wasm under warp-host |
|---|---|---|
| import strlen from "c"; strlen("hello") | 5 | 5 (hand-linked libc) |
| import atof from "c" | 5.67 | 5.67 (hand-linked) |
| import fmin from 'm' | 2.1 | 2.1 |
| erf foreign line | 0.8427 | 0.8427 |
| use m; ceil(3.2) | 4 | 4 |
| import getenv from "c" | "/home/box" | **segfault** (i32 address passed as a number): fixed on the branch by reading warp.c_calls `t` letters |
| use sqlite3 / import compressBound from "z" | "module not found" (no headers on the box) | no wasm |

## 3. Pure pipeline before the branch
Every C call was refused: `Error("outside the core: the call strlen(…)")` (also atof, getenv, erf, fmin, ceil,
sqlite3_open, compressBound) [M]. `is_import` statements are dropped (lib/compiler/emitter.warp:250-252), foreign
lines are read only for result kinds (builtin_signatures.warp:159), and module.imports had only fd_write and host words
(emitter.warp:1102-1106).
