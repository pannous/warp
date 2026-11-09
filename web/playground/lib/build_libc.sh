#!/bin/sh
# builds libc.wasm (P147, notes/wasm_modules.md) from wasi-libc, and z.wasm (`use z`, lib/native/z.warp) from zlib's
# sources, fetched once into data/zlib: brew install llvm lld wasi-libc wasi-runtimes
cd "$(dirname "$0")" || exit 1
LIBC_EXPORTS="strcmp strstr strchr strrchr strpbrk strdup toupper tolower isalpha isdigit isalnum isspace isupper islower
	ispunct isxdigit atoi atoll atof abs llabs rand srand malloc free
	strtod sqrtf fabsf ldexp ldexpf erf erfc tgamma lgamma strerror exp2"
ZLIB_VERSION=1.3.1
ZLIB=../../../data/zlib/zlib-$ZLIB_VERSION
ZLIB_EXPORTS="zlibVersion compressBound adler32 crc32 compress uncompress malloc free"
ZLIB_SOURCES="adler32 crc32 compress uncompr deflate inflate inftrees inffast trees zutil"

# wasm_library <exports> <output> <clang arguments…>: a reactor module exporting those functions
wasm_library() {
	exports=$1 output=$2
	shift 2
	PATH=/opt/homebrew/opt/lld/bin:$PATH /opt/homebrew/opt/llvm/bin/clang --target=wasm32-wasip1 \
		--sysroot=/opt/homebrew/share/wasi-sysroot -resource-dir=/opt/homebrew/opt/wasi-runtimes/share/wasi-runtimes -O2 -nostartfiles \
		-Wl,--no-entry $(for name in $exports; do printf -- '-Wl,--export=%s ' "$name"; done) -o "$output" "$@"
}

wasm_library "$LIBC_EXPORTS" libc.wasm libc.c || exit 1
[ -d "$ZLIB" ] || { mkdir -p "$(dirname "$ZLIB")" &&
	curl -sL "https://github.com/madler/zlib/releases/download/v$ZLIB_VERSION/zlib-$ZLIB_VERSION.tar.gz" | tar xz -C "$(dirname "$ZLIB")"; } || exit 1
wasm_library "$ZLIB_EXPORTS" z.wasm -DHAVE_UNISTD_H $(for source in $ZLIB_SOURCES; do printf '%s/%s.c ' "$ZLIB" "$source"; done)
