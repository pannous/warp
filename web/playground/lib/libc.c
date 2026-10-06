// libc for the browser host (P147): wasi-libc's text, character and number functions as one core module, which host.js
// calls for a program's `use c` / `import f from "c"` (natively warp opens the system libc). Built by build_libc.sh.
// The exports have the C headers' types as the native FFI sees them (size_t and long are 64 bits there, 32 in wasm32):
// those few go through wrappers, the rest are exported as they are (build_libc.sh EXPORTS)
#include <ctype.h>
#include <stdlib.h>
#include <string.h>

#define EXPORT(name) __attribute__((export_name(#name)))

EXPORT(strlen) long long warp_strlen(const char *text) { return strlen(text); }
EXPORT(strncmp) int warp_strncmp(const char *a, const char *b, long long count) { return strncmp(a, b, count); }
EXPORT(strspn) long long warp_strspn(const char *text, const char *accepted) { return strspn(text, accepted); }
EXPORT(strcspn) long long warp_strcspn(const char *text, const char *rejected) { return strcspn(text, rejected); }
EXPORT(atol) long long warp_atol(const char *text) { return atoll(text); }
EXPORT(labs) long long warp_labs(long long number) { return llabs(number); }
