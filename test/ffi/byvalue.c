#include <stdint.h>
typedef struct { uint64_t a, b, c; } Triple;
int64_t sum24(int64_t n, Triple value) { return n + (int64_t)value.a + (int64_t)value.b + (int64_t)value.c; }
int64_t pair24(void* left, void* right, Triple value) {
	return (left ? 1 : 0) + (right ? 2 : 0) + (int64_t)value.a + (int64_t)value.b + (int64_t)value.c;
}
