// libc for the browser host (P147): wasi-libc's text, character and number functions as one core module, which host.js
// calls for a program's `use c` / `import f from "c"` (natively warp opens the system libc). Built by build_libc.sh.
// The exports have the C headers' types as the native FFI sees them (size_t and long are 64 bits there, 32 in wasm32):
// those few go through wrappers, the rest are exported as they are (build_libc.sh EXPORTS)
#include <ctype.h>
#include <math.h>
#include <stdlib.h>
#include <string.h>

#define EXPORT(name) __attribute__((export_name(#name)))

EXPORT(strlen) long long warp_strlen(const char *text) { return strlen(text); }
EXPORT(strncmp) int warp_strncmp(const char *a, const char *b, long long count) { return strncmp(a, b, count); }
EXPORT(strspn) long long warp_strspn(const char *text, const char *accepted) { return strspn(text, accepted); }
EXPORT(strcspn) long long warp_strcspn(const char *text, const char *rejected) { return strcspn(text, rejected); }
EXPORT(atol) long long warp_atol(const char *text) { return atoll(text); }
EXPORT(labs) long long warp_labs(long long number) { return llabs(number); }
EXPORT(strtol) long long warp_strtol(const char *text, char **end, int base) { return strtoll(text, end, base); }

// A program in the page is no process of the system: it runs as process PROGRAM_PROCESS, the page its parent, and has
// an environment of its own (wasi-libc's would ask the host's WASI, which the page refuses)
#define PAGE_PROCESS 1
#define PROGRAM_PROCESS 2
#define ENVIRONMENT_SIZE 64
EXPORT(getpid) int warp_getpid(void) { return PROGRAM_PROCESS; }
EXPORT(getppid) int warp_getppid(void) { return PAGE_PROCESS; }

static char *environment[ENVIRONMENT_SIZE]; // "NAME=value" each, malloc'd
static int names(const char *entry, const char *name) {
	size_t length = strlen(name);
	return entry && strncmp(entry, name, length) == 0 && entry[length] == '=';
}
// the slot of the variable, else a free one, else none
static char **slot_of(const char *name) {
	char **free_slot = 0;
	for (int i = 0; i < ENVIRONMENT_SIZE; i++) {
		if (names(environment[i], name)) return &environment[i];
		if (!environment[i] && !free_slot) free_slot = &environment[i];
	}
	return free_slot;
}
EXPORT(getenv) char *warp_getenv(const char *name) {
	char **slot = slot_of(name);
	return slot && *slot ? *slot + strlen(name) + 1 : 0;
}
EXPORT(setenv) int warp_setenv(const char *name, const char *value, int overwrite) {
	char **slot = slot_of(name);
	if (!slot) return -1;
	if (*slot && !overwrite) return 0;
	char *entry = malloc(strlen(name) + strlen(value) + 2);
	if (!entry) return -1;
	strcpy(entry, name);
	strcat(entry, "=");
	strcat(entry, value);
	free(*slot);
	*slot = entry;
	return 0;
}
EXPORT(unsetenv) int warp_unsetenv(const char *name) {
	char **slot = slot_of(name);
	if (slot && *slot) {
		free(*slot);
		*slot = 0;
	}
	return 0;
}

// memory a program hands C to fill and its fields by offset and C type (lib/c.warp calloc, field_*, set_field_*;
// natively lib/native/foreign.c), counts and offsets 64 bits as there
#include <stdint.h>
EXPORT(calloc) void *warp_calloc(long long count, long long size) { return calloc(count, size); }
EXPORT(memcpy) void *warp_memcpy(void *into, const void *from, long long size) { return memcpy(into, from, size); }
#define FIELD_READER(name, type, result) EXPORT(name) result warp_##name(const uint8_t *address, long long offset) { return (result)*(const type *)(address + offset); }
#define FIELD_WRITER(name, type, value_type) EXPORT(name) void warp_##name(uint8_t *address, long long offset, value_type value) { *(type *)(address + offset) = (type)value; }
FIELD_WRITER(set_field_i32, int32_t, long long)
FIELD_WRITER(set_field_i64, int64_t, long long)
FIELD_WRITER(set_field_f64, double, double)
FIELD_WRITER(set_field_pointer, void *, void *)
FIELD_READER(field_u8, uint8_t, int32_t)
FIELD_READER(field_i16, int16_t, int32_t)
FIELD_READER(field_i32, int32_t, int32_t)
FIELD_READER(field_u32, uint32_t, long long)
FIELD_READER(field_i64, int64_t, long long)
FIELD_READER(field_f32, float, double)
FIELD_READER(field_f64, double, double)
FIELD_READER(field_pointer, void *, void *)
EXPORT(field_text) const char *warp_field_text(const uint8_t *address, long long offset) { return (const char *)(address + offset); }
// count bytes at once, a `bytes` result: {i64 length, the bytes}, freed after host-foreign.js read it
struct bytes_result { int64_t length; uint8_t bytes[]; };
EXPORT(field_bytes) struct bytes_result *warp_field_bytes(const uint8_t *address, long long offset, long long count) {
	struct bytes_result *result = malloc(sizeof *result + (size_t)count);
	result->length = count;
	memcpy(result->bytes, address + offset, (size_t)count);
	return result;
}
EXPORT(set_field_bytes) void warp_set_field_bytes(uint8_t *address, long long offset, const uint8_t *data, long long size) { memcpy(address + offset, data, (size_t)size); }
