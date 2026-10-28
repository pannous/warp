// The C functions a primitive program imports (`import getenv from "c"`), in a native build: each import wasm2c calls
// is written by core/extensions/primitive.warp c_function_glue, which finds the function by name in the process and
// passes the module's values over with these. Included behind lib/native/wasi.c.
#include <dlfcn.h>
#include <stdint.h>

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
// a text of the module, the address of its (address, length) pair, as a NUL-terminated copy the caller frees
static char* native_text(struct w2c_wasi__snapshot__preview1* wasi, uint32_t text) {
	return strndup((char*)at(wasi, load32(wasi, text)), load32(wasi, text + 4));
}
// a C text copied into the module's arena as a text, its bytes behind its pair; NULL the null address
static uint32_t module_text(struct w2c_wasi__snapshot__preview1* wasi, const char* text) {
	if (!text) return 0;
	uint32_t length = (uint32_t)strlen(text);
	uint32_t* top = w2c_program_arena(wasi->program);
	uint32_t pair = *top;
	*top = (pair + TEXT_RECORD + length + ARENA_ALIGNMENT - 1) & -ARENA_ALIGNMENT;
	if (*top > wasi->memory->size) wasm_rt_grow_memory(wasi->memory, (*top - wasi->memory->size + PAGE_BYTES - 1) / PAGE_BYTES);
	store32(wasi, pair, pair + TEXT_RECORD);
	store32(wasi, pair + 4, length);
	memcpy(at(wasi, pair + TEXT_RECORD), text, length);
	return pair;
}
