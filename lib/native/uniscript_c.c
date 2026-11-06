// The uniscript package's C library as warp calls it (lib/native/uniscript_c.warp, lib/uniscript.warp): the native twin
// of its WebAssembly module uniscript_c.wasm, whose uniscript_error and uniscript_warnings the crate's native build leaves
// out (they are WebAssembly only) and whose uniscript_to_unicode prints no warnings (the native one writes them to
// stderr, and the binding warns of each itself); these three read uniscript_convert's result. The other functions are
// the crate's own (libuniscript_ffi, which this library links; a lookup in this library finds its own first). scripts/link-packages.sh builds it beside warp:
//   cc -shared -fPIC -I<uniscript>/c uniscript_c.c -L<target> -luniscript_ffi -o libuniscript_c.so
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "uniscript.h"

// uniscript → Unicode, NULL on an error; its warnings are uniscript_warnings'
char* uniscript_to_unicode(const char* source) {
	uniscript_result result = uniscript_convert(source, UNISCRIPT_WARN);
	char* text = result.text ? strdup(result.text) : NULL;
	uniscript_result_free(&result);
	return text;
}
// the error converting the source gives ("unknown uniscript entity: x"), NULL when it converts
char* uniscript_error(const char* source) {
	uniscript_result result = uniscript_convert(source, UNISCRIPT_WARN);
	char* error = result.error ? strdup(result.error) : NULL;
	uniscript_result_free(&result);
	return error;
}
// the warnings converting the source gives, one a line as "uniscript: <message> at byte N"; "" without any
char* uniscript_warnings(const char* source) {
	uniscript_result result = uniscript_convert(source, UNISCRIPT_WARN);
	size_t size = 1;
	for (size_t i = 0; i < result.warning_count; i++) size += strlen(result.warnings[i].message) + 48;
	char* lines = malloc(size);
	size_t at = 0;
	lines[0] = 0;
	for (size_t i = 0; i < result.warning_count; i++)
		at += snprintf(lines + at, size - at, "%suniscript: %s at byte %zu", i ? "\n" : "", result.warnings[i].message, result.warnings[i].at);
	uniscript_result_free(&result);
	return lines;
}
