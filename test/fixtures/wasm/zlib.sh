#!/bin/sh
# builds zlib.wasm from zlib's own sources (libz-sys in the cargo registry carries them; else ZLIB_SRC=<zlib checkout>)
# with wasi-libc (brew install llvm lld wasi-libc wasi-runtimes): no entry point, no WASI imports
cd "$(dirname "$0")" || exit 1
ZLIB_SRC=${ZLIB_SRC:-$(ls -d ~/.cargo/registry/src/*/libz-sys-*/src/zlib | tail -1)}
SOURCES="adler32 crc32 compress uncompr deflate inflate inftrees inffast trees zutil"
EXPORTS="zlibVersion crc32 adler32 compressBound compress uncompress malloc free"
PATH=/opt/homebrew/opt/lld/bin:$PATH /opt/homebrew/opt/llvm/bin/clang --target=wasm32-wasip1 \
	--sysroot=/opt/homebrew/share/wasi-sysroot -resource-dir=/opt/homebrew/opt/wasi-runtimes/share/wasi-runtimes -O2 -nostartfiles \
	-I"$ZLIB_SRC" -Wl,--no-entry $(for name in $EXPORTS; do printf -- '-Wl,--export=%s ' "$name"; done) -o zlib.wasm \
	$(for name in $SOURCES; do printf -- '%s/%s.c ' "$ZLIB_SRC" "$name"; done)
