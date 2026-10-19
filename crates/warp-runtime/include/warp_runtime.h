/* libwarp_runtime: warp's runtime as a C library (crates/warp-runtime/src/library.rs). From warp:
   use warp_runtime
   run_wasm(bytes)        // bytes: a list of byte values or a text; gives main's result as a text */
#include <stddef.h>
#include <stdint.h>

/* Compile and run the WebAssembly module of `length` bytes with warp's host words, libm, libc and print: its main's
   result as a text ("" for none), NULL when it fails (the reason on stderr). What the module prints goes to stdout. */
const char *run_wasm(const uint8_t *module, size_t length);
