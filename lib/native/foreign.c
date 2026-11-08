// The C functions a primitive program imports (`import getenv from "c"`), in a native build: each import wasm2c calls
// is written by extensions/primitive.warp c_function_glue, which finds the function by name in the process and
// passes the module's values over with these. Included behind lib/native/wasi.c.
#include <errno.h>
#include <arpa/inet.h>
#include <dlfcn.h>
#include <netinet/in.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/socket.h>
#include <time.h>
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
// a pointer C passed to a callback (lib/callbacks.warp), returned as a handle, which the driver makes its id
void* pointer_handle(int64_t address) { return (void*)(intptr_t)address; }
// The C functions a warp function behind lib/callbacks.warp c_callback is passed to C as, one a slot: each calls the
// driver's c_callback_answered(slot, its six integer arguments), which calls the program's c_callback_fired; float
// arguments never reach one (they come in other registers)
typedef uint64_t (*callback_answer_t)(w2c_program*, u64, u64, u64, u64, u64, u64, u64);
static uint64_t callback_answered(uint64_t slot, uint64_t a, uint64_t b, uint64_t c, uint64_t d, uint64_t e, uint64_t f) {
	static callback_answer_t answer;
	if (!answer) answer = (callback_answer_t)native_function("w2c_program_c_callback_answered");
	return answer(running_wasi->program, slot, a, b, c, d, e, f);
}
#define TRAMPOLINE(n) static uint64_t trampoline_##n(uint64_t a, uint64_t b, uint64_t c, uint64_t d, uint64_t e, uint64_t f) { return callback_answered(n, a, b, c, d, e, f); }
TRAMPOLINE(0) TRAMPOLINE(1) TRAMPOLINE(2) TRAMPOLINE(3) TRAMPOLINE(4) TRAMPOLINE(5) TRAMPOLINE(6) TRAMPOLINE(7)
TRAMPOLINE(8) TRAMPOLINE(9) TRAMPOLINE(10) TRAMPOLINE(11) TRAMPOLINE(12) TRAMPOLINE(13) TRAMPOLINE(14) TRAMPOLINE(15)
static void* trampolines[] = {trampoline_0, trampoline_1, trampoline_2, trampoline_3, trampoline_4, trampoline_5, trampoline_6, trampoline_7,
	trampoline_8, trampoline_9, trampoline_10, trampoline_11, trampoline_12, trampoline_13, trampoline_14, trampoline_15};
void* c_callback_pointer(int64_t slot) {
	if (slot < 0 || slot >= (int64_t)(sizeof trampolines / sizeof *trampolines)) {
		fprintf(stderr, "c_callback: at most %zu callbacks a run\n", sizeof trampolines / sizeof *trampolines);
		exit(1);
	}
	return trampolines[slot];
}
// paint's count in this run, from 1: its PNG's number (core/native/driver.warp painted)
int32_t next_painting(void) { static int32_t painted; return ++painted; }
// the error number of the C call that failed last (core/native/driver.warp os_failure)
int32_t last_error(void) { return errno; }
void* library_function(char* library, char* name);
// whether a shader paints on the GPU: libwgpu_native beside warp (lib/host/native_host/gpu.warp paints through it),
// else (none, or WARP_GPU=off) its function on the CPU (lib/prelude.warp paint_shader)
int32_t gpu_paints(void) {
	static int32_t asked, present;
	if (!asked) {
		asked = 1;
		const char* chosen = getenv("WARP_GPU");
		present = !(chosen && !strcmp(chosen, "off")) && library_function("wgpu_native", "wgpuCreateInstance");
	}
	return present;
}
// the terminal's rows, else its columns, on standard output; 0 outside a terminal (ioctl is variadic: called here, not
// through native_call)
int32_t terminal_size(int32_t rows) {
	struct winsize size;
	return ioctl(STDOUT_FILENO, TIOCGWINSZ, &size) == 0 ? (rows ? size.ws_row : size.ws_col) : 0;
}
// what the driver keeps between host calls, by slot (driver.warp's *_SLOT: the system values a program set), 0 unset
static int64_t kept_values[8];
int64_t kept_value(int64_t slot) { return kept_values[slot]; }
void keep_value(int64_t slot, int64_t value) { kept_values[slot] = value; }
// whether a route to the internet exists: a UDP connect, which sends nothing
int32_t route_online(void) {
	int udp = socket(AF_INET, SOCK_DGRAM, 0);
	if (udp < 0) return 0;
	struct sockaddr_in public_resolver = {.sin_family = AF_INET, .sin_port = htons(53)};
	inet_pton(AF_INET, "8.8.8.8", &public_resolver.sin_addr);
	int routed = connect(udp, (struct sockaddr*)&public_resolver, sizeof public_resolver) == 0;
	close(udp);
	return routed;
}
// the main screen's width, else its height, in points; 0 where none shows (a headless Mac, Linux without CoreGraphics)
int64_t display_extent(int32_t height) {
#ifdef __APPLE__
	void* graphics = dlopen("/System/Library/Frameworks/CoreGraphics.framework/CoreGraphics", RTLD_LAZY | RTLD_LOCAL);
	if (!graphics) return 0;
	uint32_t (*main_display)(void) = (uint32_t (*)(void))dlsym(graphics, "CGMainDisplayID");
	size_t (*extent)(uint32_t) = (size_t (*)(uint32_t))dlsym(graphics, height ? "CGDisplayPixelsHigh" : "CGDisplayPixelsWide");
	return main_display && extent ? (int64_t)extent(main_display()) : 0;
#else
	(void)height;
	return 0;
#endif
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
// libraries beside the executable first (libwgpu_native), as libwarp_wasmtime is
// Fixed short folders per platform (user 2026-11-06 library-search-paths): no scan/sort/computed from.
// macOS: homebrew + local + opt/<name>/Frameworks/<Name>.framework/<Name> + the system's frameworks. Linux:
// /usr/lib(+triple) + local, lib<name>.so / .so.0 / -2.0.so.0 / .so.1 / .so.6 (usual versioned sonames, FreeType's
// .so.6; odd ones like libpython3.13.so.1.0 still
// need a symlink, WARP path, or a computed `from`). Windows: LoadLibrary System32+PATH (not this dlopen list).
static const char* LIBRARY_PATHS[] = {
	"@executable_path/lib%s.dylib",
	"$ORIGIN/lib%s.so",
	"lib%s.dylib",
	"lib%s.so",
	"lib%s.so.0",
	"lib%s-2.0.so.0",
	"lib%s-2.so.0",
	"lib%s.so.1",
	"lib%s.so.1.0",
	"lib%s.so.6",
	"lib%s3.so",
	"/opt/homebrew/lib/lib%s.dylib",
	"/usr/local/lib/lib%s.dylib",
	"/usr/local/lib/lib%s.so",
	"/usr/local/lib/lib%s-2.0.so.0",
	"/usr/local/lib/lib%s3.so",
	"/usr/lib/lib%s.so",
	"/usr/lib/lib%s-2.0.so.0",
	"/usr/lib/lib%s3.so",
	"/usr/lib/x86_64-linux-gnu/lib%s.so",
	"/usr/lib/x86_64-linux-gnu/lib%s.so.0",
	"/usr/lib/x86_64-linux-gnu/lib%s-2.0.so.0",
	"/usr/lib/x86_64-linux-gnu/lib%s.so.1",
	"/usr/lib/x86_64-linux-gnu/lib%s3.so",
	"/usr/lib/aarch64-linux-gnu/lib%s.so",
	"/usr/lib/aarch64-linux-gnu/lib%s.so.0",
	"/usr/lib/aarch64-linux-gnu/lib%s-2.0.so.0",
	"/usr/lib/aarch64-linux-gnu/lib%s.so.1",
	"/usr/lib/aarch64-linux-gnu/lib%s3.so",
};
static const char* PROCESS_LIBRARIES[] = {"c", "m", "libc", "libm"};
// a library's function, NULL when the library has none of that name: never another library's
void* library_function(char* library, char* name) {
	for (size_t i = 0; i < sizeof PROCESS_LIBRARIES / sizeof *PROCESS_LIBRARIES; i++)
		if (!strcmp(library, PROCESS_LIBRARIES[i])) return dlsym(RTLD_DEFAULT, name);
	// global: a library's own plugins (python's extension modules) find its symbols; dlsym on the handle still answers
	// from this library alone
	int flags = RTLD_NOW | RTLD_GLOBAL;
	// path as given: contains '/', dlopen that file
	if (strchr(library, '/')) {
		void* loaded = dlopen(library, flags);
		return loaded ? dlsym(loaded, name) : NULL;
	}
	char path[4096];
	for (size_t i = 0; i < sizeof LIBRARY_PATHS / sizeof *LIBRARY_PATHS; i++) {
		snprintf(path, sizeof path, LIBRARY_PATHS[i], library);
		void* loaded = dlopen(path, flags);
		if (loaded) return dlsym(loaded, name);
	}
#ifdef __APPLE__
	// Homebrew framework: /opt/homebrew/opt/<name>/Frameworks/<Name>.framework/<Name>
	{
		char Name[256];
		size_t n = strlen(library);
		if (n > 0 && n < sizeof Name) {
			memcpy(Name, library, n + 1);
			if (Name[0] >= 'a' && Name[0] <= 'z') Name[0] = (char)(Name[0] - 32);
			snprintf(path, sizeof path, "/opt/homebrew/opt/%s/Frameworks/%s.framework/%s", library, Name, Name);
			void* loaded = dlopen(path, flags);
			if (loaded) return dlsym(loaded, name);
		}
	}
	// a system framework by its name: `from 'CoreFoundation'`
	snprintf(path, sizeof path, "/System/Library/Frameworks/%s.framework/%s", library, library);
	void* framework = dlopen(path, flags);
	if (framework) return dlsym(framework, name);
#endif
	return NULL;
}
// A field of a C struct a program holds as a handle, by its offset (lib/c.warp: SDL_Event's union, FreeType's glyph
// slot): its bytes as the C type is, a signed integer widened, a float as a double, a pointer as a handle; set_field_*
// writes one (an AudioStreamBasicDescription, an AudioBufferList pointing at memory of the program's)
#define FIELD_READER(name, type, result) result name(const uint8_t* address, int64_t offset) { type value; memcpy(&value, address + offset, sizeof value); return (result)value; }
#define FIELD_WRITER(name, type, given) void name(uint8_t* address, int64_t offset, given value) { type field = (type)value; memcpy(address + offset, &field, sizeof field); }
FIELD_WRITER(set_field_i32, int32_t, int64_t)
FIELD_WRITER(set_field_i64, int64_t, int64_t)
FIELD_WRITER(set_field_f64, double, double)
FIELD_WRITER(set_field_pointer, void*, void*)
FIELD_READER(field_u8, uint8_t, int32_t)
FIELD_READER(field_i16, int16_t, int32_t)
FIELD_READER(field_i32, int32_t, int32_t)
FIELD_READER(field_u32, uint32_t, int64_t)
FIELD_READER(field_i64, int64_t, int64_t)
FIELD_READER(field_f32, float, double)
FIELD_READER(field_f64, double, double)
FIELD_READER(field_pointer, void*, void*)
const char* field_text(const uint8_t* address, int64_t offset) { return (const char*)(address + offset); }
// count bytes of a struct or buffer at once, a `bytes` result: {length, bytes} in memory the driver copies into a text
// and frees; set_field_bytes writes a text's bytes there (a mapped GPU buffer read back, a vertex array filled)
struct bytes_result { int64_t length; uint8_t bytes[]; };
struct bytes_result* field_bytes(const uint8_t* address, int64_t offset, int64_t count) {
	struct bytes_result* result = malloc(sizeof *result + (size_t)count);
	result->length = count;
	memcpy(result->bytes, address + offset, (size_t)count);
	return result;
}
void set_field_bytes(uint8_t* address, int64_t offset, const uint8_t* data, int64_t size) { memcpy(address + offset, data, (size_t)size); }
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

// what a host word allocated given back, zeroed: the arena past its top stays as fresh memory is, so an allocate after
// a host word (warp test's slots after test-list ran) never starts with an earlier word's bytes. What arena_kept
// kept stays: a module the driver links on a host word's question (driver.warp lazily_linked) keeps its records
static uint32_t arena_floor;
void arena_kept(void) { arena_floor = *w2c_program_arena(running_wasi->program); }
static void arena_given_back(struct w2c_wasi__snapshot__preview1* wasi, uint32_t before) {
	uint32_t* top = w2c_program_arena(wasi->program);
	uint32_t kept = before > arena_floor ? before : arena_floor;
	if (*top > kept) memset(at(wasi, kept), 0, *top - kept);
	*top = kept;
}
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
	arena_given_back(wasi, before);
	return NULL;
}
