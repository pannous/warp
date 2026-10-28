// The native program around a primitive module (test_native.sh): instantiate it, print what its main gives
#include <inttypes.h>
#include <stdio.h>
#include "program.h"

int main(void) {
	wasm_rt_init();
	w2c_program program;
	wasm2c_program_instantiate(&program);
	printf("%" PRId64 "\n", (int64_t)w2c_program_main(&program));
	wasm2c_program_free(&program);
	wasm_rt_free();
	return 0;
}
