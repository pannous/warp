// The C functions a primitive program imports (`import getenv from "c"`), in a native build: each import wasm2c calls
// is written by core/extensions/primitive.warp c_function_glue, which finds the function by name in the process and
// passes the module's values over with these. Included behind lib/native/wasi.c.
#include <dlfcn.h>
#include <stdint.h>
#include <sys/ioctl.h>
#include <unistd.h>

enum { ARENA_ALIGNMENT = 8, TEXT_RECORD = 8, PAGE_BYTES = 65536 }; // primitive.warp ARENA_ALIGNMENT, text_layout's size

// the C function of that name the executable or a library it links has; a program calling one it lacks stops here
static void* native_function(const char* name) {
	void* found = dlsym(RTLD_DEFAULT, name);
	if (!found) {
		fprintf(stderr, "no C function %s in this executable or the libraries it links\n", name);
		exit(1);
	}
	return found;
}
// an address of the module's memory as a C pointer, the null address NULL; and back
static void* native_address(struct w2c_wasi__snapshot__preview1* wasi, uint32_t address) { return address ? at(wasi, address) : NULL; }
static uint32_t module_address(struct w2c_wasi__snapshot__preview1* wasi, void* pointer) {
	return pointer ? (uint32_t)((uint8_t*)pointer - wasi->memory->data) : 0;
}
// an address of the module's memory as the C pointer it is, for a program to hold as a handle (`native_pointer(address:
// pointer): handle from "c"`): the glue made it the pointer already
void* native_pointer(void* address) { return address; }
// paint's count in this run, from 1: its PNG's number (core/native/driver.warp painted)
int32_t next_painting(void) { static int32_t painted; return ++painted; }
// the terminal's rows, else its columns, on standard output; 0 outside a terminal (ioctl is variadic: called here, not
// through native_call)
int32_t terminal_size(int32_t rows) {
	struct winsize size;
	return ioctl(STDOUT_FILENO, TIOCGWINSZ, &size) == 0 ? (rows ? size.ws_row : size.ws_col) : 0;
}
// a text of the module, the address of its (address, length) pair, as a NUL-terminated copy the caller frees
static char* native_text(struct w2c_wasi__snapshot__preview1* wasi, uint32_t text) {
	return strndup((char*)at(wasi, load32(wasi, text)), load32(wasi, text + 4));
}
// bytes taken from the module's arena (primitive.warp arena_taken), its memory grown when they pass its end
static uint32_t arena_bytes(struct w2c_wasi__snapshot__preview1* wasi, uint32_t size) {
	uint32_t* top = w2c_program_arena(wasi->program);
	uint32_t taken = *top;
	*top = (taken + size + ARENA_ALIGNMENT - 1) & -ARENA_ALIGNMENT;
	if (*top > wasi->memory->size) wasm_rt_grow_memory(wasi->memory, (*top - wasi->memory->size + PAGE_BYTES - 1) / PAGE_BYTES);
	return taken;
}
// a C text copied into the module's arena as a text, its bytes behind its pair; NULL the null address
static uint32_t module_text(struct w2c_wasi__snapshot__preview1* wasi, const char* text) {
	if (!text) return 0;
	uint32_t length = (uint32_t)strlen(text);
	uint32_t pair = arena_bytes(wasi, TEXT_RECORD + length);
	store32(wasi, pair, pair + TEXT_RECORD);
	store32(wasi, pair + 4, length);
	memcpy(at(wasi, pair + TEXT_RECORD), text, length);
	return pair;
}

// a C function's address by its name, for a program to hand to a C library as a callback
void* c_function(char* name) { return native_function(name); }

// The C calls of a module the driver runs (core/native/driver.warp c_called), as the Rust stub makes them (c_calls.rs)
// where a library lies by the name a program imports it under (`sqlite3`); libc's and libm's are the process's
static const char* LIBRARY_PATHS[] = {"lib%s.dylib", "/opt/homebrew/lib/lib%s.dylib", "/usr/local/lib/lib%s.dylib", "lib%s.so", "lib%s.so.0"};
static const char* PROCESS_LIBRARIES[] = {"c", "m", "libc", "libm"};
// a library's function, NULL when the library has none of that name: never another library's
void* library_function(char* library, char* name) {
	for (size_t i = 0; i < sizeof PROCESS_LIBRARIES / sizeof *PROCESS_LIBRARIES; i++)
		if (!strcmp(library, PROCESS_LIBRARIES[i])) return dlsym(RTLD_DEFAULT, name);
	char path[4096];
	for (size_t i = 0; i < sizeof LIBRARY_PATHS / sizeof *LIBRARY_PATHS; i++) {
		snprintf(path, sizeof path, LIBRARY_PATHS[i], library);
		void* loaded = dlopen(path, RTLD_NOW | RTLD_LOCAL);
		if (loaded) return dlsym(loaded, name);
	}
	return NULL;
}
// a C function called with up to 8 integers and pointers and up to 8 floats in their registers (the ARM64 and x86-64
// conventions keep the two apart, `ldexp(double, int)` takes d0 and x0): it reads only the ones its parameters name.
// Its result is the integer register, or the bits of the float register
typedef uint64_t (*integer_function)(uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, double, double, double, double, double, double, double, double);
typedef double (*float_function)(uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, double, double, double, double, double, double, double, double);
uint64_t native_call(void* function, uint64_t* i, double* f, int32_t float_result) {
	if (!float_result) return ((integer_function)function)(i[0], i[1], i[2], i[3], i[4], i[5], i[6], i[7], f[0], f[1], f[2], f[3], f[4], f[5], f[6], f[7]);
	double result = ((float_function)function)(i[0], i[1], i[2], i[3], i[4], i[5], i[6], i[7], f[0], f[1], f[2], f[3], f[4], f[5], f[6], f[7]);
	uint64_t bits;
	memcpy(&bits, &result, sizeof bits);
	return bits;
}
// the C pointers a run got, handed to the module as ids: id n is the n-th pointer, 0 is NULL, the same pointer keeps
// its id, found again through a table of ids by pointer (open addressing, at most half full)
static void** handles;
static int32_t* handle_ids;
static uint32_t handle_count, handle_room, slot_count;
static int32_t* slot_of(void* pointer) {
	uint32_t mask = slot_count - 1;
	for (uint32_t slot = (uint32_t)(((uintptr_t)pointer * 0x9E3779B97F4A7C15ull) >> 32) & mask;; slot = (slot + 1) & mask)
		if (!handle_ids[slot] || handles[handle_ids[slot] - 1] == pointer) return &handle_ids[slot];
}
int32_t handle_id(void* pointer) {
	if (!pointer) return 0;
	if (handle_count * 2 >= slot_count) {
		slot_count = slot_count ? slot_count * 2 : 64;
		free(handle_ids);
		handle_ids = calloc(slot_count, sizeof *handle_ids);
		for (uint32_t id = 1; id <= handle_count; id++) *slot_of(handles[id - 1]) = (int32_t)id;
	}
	int32_t* slot = slot_of(pointer);
	if (*slot) return *slot;
	if (handle_count == handle_room) handles = realloc(handles, (handle_room = handle_room * 2 + 16) * sizeof *handles);
	handles[handle_count++] = pointer;
	return *slot = (int32_t)handle_count;
}
int32_t handle_known(int32_t id) { return id >= 0 && (uint32_t)id <= handle_count; }
void* handle_pointer(int32_t id) { return id > 0 && (uint32_t)id <= handle_count ? handles[id - 1] : NULL; }

// A host word a program defines for a module it runs through wasmtime's C API (core/native/driver.warp
// host_words_defined): a wasmtime_func_callback_t whose data is the word's number. The values (wasmtime_val_t, VALUE_BYTES
// each) cross through the program's arena to its exported host_word(word, caller, values, count, results, count),
// whose arena is given back after
enum { VALUE_BYTES = 32 };
typedef void (*host_word_t)(w2c_program*, u64, u64, u32, u64, u32, u64);
void* warp_host_callback(void* word, void* caller, const void* values, size_t count, void* results, size_t results_count) {
	static host_word_t answer;
	if (!answer) answer = (host_word_t)native_function("w2c_program_host_word");
	struct w2c_wasi__snapshot__preview1* wasi = running_wasi;
	uint32_t* top = w2c_program_arena(wasi->program);
	uint32_t before = *top;
	uint32_t given = arena_bytes(wasi, (uint32_t)(count * VALUE_BYTES));
	uint32_t answered = arena_bytes(wasi, (uint32_t)(results_count * VALUE_BYTES));
	memcpy(at(wasi, given), values, count * VALUE_BYTES);
	answer(wasi->program, (u64)(uintptr_t)word, (u64)(uintptr_t)caller, given, count, answered, results_count);
	memcpy(results, at(wasi, answered), results_count * VALUE_BYTES);
	*top = before;
	return NULL;
}
