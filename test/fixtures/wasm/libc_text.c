// libc hijacked: wasi-libc's text and number functions as one core WebAssembly module (tests/modules/test_wasm_modules.rs),
// built by `sh tests/fixtures/wasm/libc_text.sh`, which exports them by name; libc_text.h declares what warp may call
#include <ctype.h>
#include <stdlib.h>
#include <string.h>
