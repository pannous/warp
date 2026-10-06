#!/bin/sh
# builds libc_text.wasm from wasi-libc (brew install llvm lld wasi-libc wasi-runtimes): no entry point, no WASI imports
cd "$(dirname "$0")" || exit 1
EXPORTS="strlen strstr strchr strrchr strtol toupper tolower atoi atol abs malloc free"
PATH=/opt/homebrew/opt/lld/bin:$PATH /opt/homebrew/opt/llvm/bin/clang --target=wasm32-wasip1 \
	--sysroot=/opt/homebrew/share/wasi-sysroot -resource-dir=/opt/homebrew/opt/wasi-runtimes/share/wasi-runtimes -O2 -nostartfiles \
	-Wl,--no-entry $(for name in $EXPORTS; do printf -- '-Wl,--export=%s ' "$name"; done) -o libc_text.wasm libc_text.c
