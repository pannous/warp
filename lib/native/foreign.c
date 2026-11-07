// The C functions a primitive program imports (`import getenv from "c"`), in a native build: each import wasm2c calls
// is written by extensions/primitive.warp c_function_glue, which finds the function by name in the process and
// passes the module's values over with these. Included behind lib/native/wasi.c.
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
// paint's count in this run, from 1: its PNG's number (core/native/driver.warp painted)
int32_t next_painting(void) { static int32_t painted; return ++painted; }
void* library_function(char* library, char* name);
// libwarp_gpu beside warp (crates/warp-gpu, card gpu-functions), found at the first question: a shader paints on the
// GPU, else (none, or WARP_GPU=off) its function on the CPU (lib/prelude.warp paint_shader)
typedef int32_t (*gpu_paint_function)(const char*, const char*, int32_t, int32_t, uint32_t*);
static gpu_paint_function gpu_paint;
static const char* (*gpu_failure)(void);
int32_t gpu_paints(void) {
	static int32_t asked;
	if (!asked) {
		asked = 1;
		const char* chosen = getenv("WARP_GPU");
		if (chosen && !strcmp(chosen, "off")) return 0;
		gpu_paint = (gpu_paint_function)library_function("warp_gpu", "warp_gpu_paint");
		gpu_failure = (const char* (*)(void))library_function("warp_gpu", "warp_gpu_failure");
	}
	return gpu_paint && gpu_failure;
}
static uint32_t module_text(struct w2c_wasi__snapshot__preview1* wasi, const char* text);
static void arena_given_back(struct w2c_wasi__snapshot__preview1* wasi, uint32_t before);
// a system value as the driver answers it (core/native/driver.warp system_value, the native host module's window
// among them), asked in a host word: what it allocates is given back after
static int64_t library_value(const char* name) {
	static uint64_t (*value_of)(w2c_program*, uint32_t);
	if (!value_of) value_of = (uint64_t (*)(w2c_program*, uint32_t))native_function("w2c_program_system_value");
	struct w2c_wasi__snapshot__preview1* wasi = running_wasi;
	uint32_t* top = w2c_program_arena(wasi->program);
	uint32_t before = *top;
	int64_t value = (int64_t)value_of(wasi->program, module_text(wasi, name));
	arena_given_back(wasi, before);
	return value;
}
// the holes every shader has, filled where the shader reads them and the program gave none, as the playground fills
// them (web/playground/host-gpu.js builtinValues): the image's size, seconds since the first painting, the paintings
// before, the pointer and key over the window (0 without one)
enum { MOST_VALUES_TEXT = 65536 };
static const char* BUILTIN_HOLES[] = {"width", "height", "size", "time", "frame", "mouse", "mouse_down", "mouse_wheel", "key"};
static int32_t reads_hole(const char* text, const char* name) {
	size_t length = strlen(name);
	for (const char* at = strstr(text, "values."); at; at = strstr(at + 1, "values.")) {
		const char* field = at + strlen("values.");
		char after = field[length];
		int boundary = !(after == '_' || (after >= 'a' && after <= 'z') || (after >= 'A' && after <= 'Z') || (after >= '0' && after <= '9'));
		if (!strncmp(field, name, length) && boundary) return 1;
	}
	return 0;
}
static int32_t given_hole(const char* values, const char* name) {
	size_t length = strlen(name);
	for (const char* at = strstr(values, name); at; at = strstr(at + 1, name))
		if ((at == values || at[-1] == '{' || at[-1] == ' ' || at[-1] == ',') && at[length] == ':') return 1;
	return 0;
}
static double seconds_now(void) {
	struct timespec now;
	clock_gettime(CLOCK_MONOTONIC, &now);
	return now.tv_sec + now.tv_nsec / 1e9;
}
static void hole_written(char* into, const char* name, int64_t width, int64_t height, double time, int64_t frame) {
	size_t used = strlen(into);
	char* at = into + used;
	size_t room = MOST_VALUES_TEXT - used;
	if (!strcmp(name, "width")) snprintf(at, room, "width: %lld", (long long)width);
	else if (!strcmp(name, "height")) snprintf(at, room, "height: %lld", (long long)height);
	else if (!strcmp(name, "size")) snprintf(at, room, "size: [%lld, %lld]", (long long)width, (long long)height);
	else if (!strcmp(name, "time")) snprintf(at, room, "time: %.6f", time);
	else if (!strcmp(name, "frame")) snprintf(at, room, "frame: %lld", (long long)frame);
	else if (!strcmp(name, "mouse")) snprintf(at, room, "mouse: [%lld, %lld]", (long long)library_value("mouse_x"), (long long)library_value("mouse_y"));
	else if (!strcmp(name, "key")) snprintf(at, room, "key: %lld", (long long)library_value("key code held"));
	else snprintf(at, room, "%s: %lld", name, (long long)library_value(name));
}
// the program's values with the built-in holes its shader reads; the caller frees it
static char* values_filled(const char* shader, const char* values, int64_t width, int64_t height) {
	static double first_painting;
	static int64_t paintings;
	if (!first_painting) first_painting = seconds_now();
	char* filled = calloc(MOST_VALUES_TEXT, 1);
	const char* given = values[0] == '{' ? values : "{}";
	size_t given_length = strlen(given);
	snprintf(filled, MOST_VALUES_TEXT, "%.*s", (int)(given_length - 1), given);
	int members = strcmp(given, "{}") != 0;
	for (size_t i = 0; i < sizeof BUILTIN_HOLES / sizeof *BUILTIN_HOLES; i++) {
		if (!reads_hole(shader, BUILTIN_HOLES[i]) || given_hole(given, BUILTIN_HOLES[i])) continue;
		if (members++) strncat(filled, ", ", MOST_VALUES_TEXT - strlen(filled) - 1);
		hole_written(filled, BUILTIN_HOLES[i], width, height, seconds_now() - first_painting, paintings);
	}
	strncat(filled, "}", MOST_VALUES_TEXT - strlen(filled) - 1);
	paintings++;
	return filled;
}
// the shader's pixels, as 0xAARRGGBB ints of 8 bytes (driver.warp png_written); 0, else -1, the GPU's reason told
int32_t gpu_painted(char* shader, char* given_values, int64_t width, int64_t height, int64_t* pixels) {
	if (!gpu_paints()) return fprintf(stderr, "paint of a shader: libwarp_gpu is not beside warp\n"), -1;
	uint32_t* colors = malloc(width * height * sizeof *colors);
	char* values = values_filled(shader, given_values, width, height);
	int32_t failed = gpu_paint(shader, values, (int32_t)width, (int32_t)height, colors);
	free(values);
	if (failed) fprintf(stderr, "paint of a shader: %s\n", gpu_failure());
	else for (int64_t i = 0; i < width * height; i++) pixels[i] = colors[i];
	free(colors);
	return failed;
}
// The host words the driver does not answer, bound to the libraries beside warp that own some (libwarp_gpu's
// gpu_compute, libwarp_webview's foreign_call; decision library-direct-refs): the wasmtime_func_callback_t
// `warp_word_<name>` a library exports, reading and building the module's values in place (lib/native/warp_node.h);
// NULL when none owns it. Two owning one name end the run naming both
static const char* HOST_WORD_LIBRARIES[] = {"warp_gpu", "warp_webview"};
enum { HOST_WORD_COUNT = sizeof HOST_WORD_LIBRARIES / sizeof *HOST_WORD_LIBRARIES, WORD_SYMBOL_ROOM = 256 };
void* library_word(char* name) {
	char symbol[WORD_SYMBOL_ROOM];
	snprintf(symbol, sizeof symbol, "warp_word_%s", name);
	void* word = NULL;
	const char* owner = NULL;
	for (int i = 0; i < HOST_WORD_COUNT; i++) {
		void* found = library_function((char*)HOST_WORD_LIBRARIES[i], symbol);
		if (!found) continue;
		if (word) {
			fprintf(stderr, "the host word %s is in two libraries: lib%s and lib%s\n", name, owner, HOST_WORD_LIBRARIES[i]);
			exit(1);
		}
		word = found;
		owner = HOST_WORD_LIBRARIES[i];
	}
	return word;
}
// A painting shown by libwarp_gpu in the layer of the native host module's open window (core/native/driver.warp
// window_layer), on the GPU throughout: 1 shown, 0 no window open (the driver writes its PNG), -1 failed, the reason told
static int32_t shown_or_told(int32_t failed) {
	if (failed) fprintf(stderr, "paint into the window: %s\n", gpu_failure());
	return failed ? -1 : 1;
}
int32_t shader_shown(char* shader, char* given_values, void* layer, int64_t width, int64_t height) {
	if (!layer) return 0;
	static int32_t (*show)(const char*, const char*, void*, int32_t, int32_t);
	if (!show) show = (int32_t (*)(const char*, const char*, void*, int32_t, int32_t))library_function("warp_gpu", "warp_gpu_show");
	char* values = values_filled(shader, given_values, width, height);
	int32_t shown = shown_or_told(show(shader, values, layer, (int32_t)width, (int32_t)height));
	free(values);
	return shown;
}
// the pixels as driver.warp png_written shades them: 0 paper, a value with an alpha byte its color, any other ink
enum { PAPER = 250, INK = 29, COLOR_FROM = 16777216 };
int32_t pixels_shown(int64_t* pixels, void* layer, int64_t width, int64_t height) {
	if (!layer) return 0;
	static int32_t (*show)(const uint8_t*, void*, int32_t, int32_t);
	if (!show) show = (int32_t (*)(const uint8_t*, void*, int32_t, int32_t))library_function("warp_gpu", "warp_gpu_show_pixels");
	uint8_t* bytes = malloc(width * height * 4);
	for (int64_t i = 0; i < width * height; i++) {
		int64_t value = pixels[i];
		uint8_t shade = value == 0 ? PAPER : INK;
		bytes[4 * i] = value >= COLOR_FROM ? (value >> 16) & 255 : shade;
		bytes[4 * i + 1] = value >= COLOR_FROM ? (value >> 8) & 255 : shade;
		bytes[4 * i + 2] = value >= COLOR_FROM ? value & 255 : shade;
		bytes[4 * i + 3] = 255;
	}
	int32_t shown = shown_or_told(show(bytes, layer, (int32_t)width, (int32_t)height));
	free(bytes);
	return shown;
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
// warp's own libraries (libwarp_gpu) beside the executable first, as libwarp_wasmtime is
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
// A host word the driver doesn't answer, owned by a library beside warp or by none, found at its first call: opening
// libwarp_gpu to look was half of hello's run once every program linked prelude.wasm, which
// imports clipboard_text. Its data: the word's name, the engine its owner reads values with, the owner once found,
// and the word naming it when none owns it (driver.warp unanswered_word)
typedef void* (*wasm_callback_t)(void*, void*, const void*, size_t, void*, size_t);
struct lazy_word { char* name; void* engine; wasm_callback_t owner; int64_t unanswered; int asked; };
void* lazy_library_word(char* name, void* engine, int64_t unanswered) {
	struct lazy_word* word = calloc(1, sizeof *word);
	word->name = strdup(name);
	word->engine = engine;
	word->unanswered = unanswered;
	return word;
}
void* warp_lazy_host_callback(void* data, void* caller, const void* values, size_t count, void* results, size_t results_count) {
	struct lazy_word* word = data;
	if (!word->asked) {
		word->asked = 1;
		word->owner = (wasm_callback_t)library_word(word->name);
	}
	if (word->owner) return word->owner(word->engine, caller, values, count, results, results_count);
	return warp_host_callback((void*)(intptr_t)word->unanswered, caller, values, count, results, results_count);
}
