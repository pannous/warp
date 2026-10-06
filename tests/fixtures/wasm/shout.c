// a C library compiled to core WebAssembly, without libc (tests/modules/test_wasm_modules.rs), built by
// PATH=/opt/homebrew/opt/lld/bin:$PATH /opt/homebrew/opt/llvm/bin/clang --target=wasm32 -nostdlib -fno-builtin -O2 -Wl,--no-entry \
//   -Wl,--export=shout -Wl,--export=letters -Wl,--export=malloc -o shout.wasm shout.c
#include "shout.h"

static unsigned char heap[4096];
static unsigned long used;

// a bump allocator, exported: warp copies a text argument into its block
void *malloc(unsigned long size);
void *malloc(unsigned long size) {
	void *block = heap + used;
	used += (size + 7) & ~7ul;
	return block;
}

int letters(const char *text) {
	int count = 0;
	while (text[count]) count++;
	return count;
}

char *shout(const char *text) {
	int length = letters(text);
	char *loud = malloc(length + 2);
	for (int index = 0; index < length; index++) {
		char letter = text[index];
		loud[index] = letter >= 'a' && letter <= 'z' ? letter - 'a' + 'A' : letter;
	}
	loud[length] = '!';
	loud[length + 1] = 0;
	return loud;
}
