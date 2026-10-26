#!/bin/sh
# builds xxhash.wasm from xxHash's header (zstd-sys in the cargo registry carries it; else XXHASH_SRC=<dir of xxhash.h>)
# with wasi-libc (brew install llvm lld wasi-libc wasi-runtimes): no entry point, no WASI imports
cd "$(dirname "$0")" || exit 1
XXHASH_SRC=${XXHASH_SRC:-$(ls -d ~/.cargo/registry/src/*/zstd-sys-*/zstd/lib/common | tail -1)}
EXPORTS="XXH32 XXH64 XXH32_createState XXH32_freeState XXH32_reset XXH32_update XXH32_digest XXH_versionNumber malloc free"
PATH=/opt/homebrew/opt/lld/bin:$PATH /opt/homebrew/opt/llvm/bin/clang --target=wasm32-wasip1 \
	--sysroot=/opt/homebrew/share/wasi-sysroot -resource-dir=/opt/homebrew/opt/wasi-runtimes/share/wasi-runtimes -O2 -nostartfiles \
	-iquote "$XXHASH_SRC" -Wl,--no-entry $(for name in $EXPORTS; do printf -- '-Wl,--export=%s ' "$name"; done) -o xxhash.wasm xxhash.c
