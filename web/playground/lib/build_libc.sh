#!/bin/sh
# builds libc.wasm (P147, notes/wasm_modules.md) from wasi-libc: brew install llvm lld wasi-libc wasi-runtimes
cd "$(dirname "$0")" || exit 1
EXPORTS="strcmp strstr strchr strrchr strpbrk strdup toupper tolower isalpha isdigit isalnum isspace isupper islower
	ispunct isxdigit atoi atoll atof abs llabs rand srand malloc free"
PATH=/opt/homebrew/opt/lld/bin:$PATH /opt/homebrew/opt/llvm/bin/clang --target=wasm32-wasip1 \
	--sysroot=/opt/homebrew/share/wasi-sysroot -resource-dir=/opt/homebrew/opt/wasi-runtimes/share/wasi-runtimes -O2 -nostartfiles \
	-Wl,--no-entry $(for name in $EXPORTS; do printf -- '-Wl,--export=%s ' "$name"; done) -o libc.wasm libc.c
