#!/usr/bin/env bash
# Rust crates called from warp programs through warp-host's plain C FFI (card ffi-rust-crate-adapter,
# notes/crate_adapter.md): scripts/warp-crate builds lib/crates/{regex,sha2,crc32fast}.warp, and semver's
# lines drafted by `warp-crate scan`, into cdylib shims
# (cached in ~/.cache/warp/crates, or WARP_CRATES_CACHE), and programs `use rs_regex` etc. Nothing of a crate is in
# warp-host. Run it through the queue: tests/queue.sh tests/host/test_crates.sh
# Needs cargo (the first build fetches and compiles the crates, ~1 min each), and a warp-host and compiler that compile
# C imports (card pure-c-imports): WARP_HOST_BINARY (default as tests/host/test_host.sh finds it), the compiler module
# WARP_CRATES_COMPILER_FILE (default compiler.wasm beside WARP_LAUNCHER_RUST), and WARP_CRATES_LIB, the lib folder
# whose compiler reads C imports (default the checkout's lib).
checkout=$(cd "$(dirname "$0")/../.." && pwd)
host=${WARP_HOST_BINARY:-$checkout/target/host/warp-host}
[ -x "$host" ] || host=$checkout/target/release/warp-host
[ -x "$host" ] || host=$checkout/target/debug/warp-host
rust=${WARP_LAUNCHER_RUST:-$checkout/target/debug/warp}
compiler_path=${WARP_CRATES_COMPILER_FILE:-$(dirname "$rust")/compiler.wasm}
library=${WARP_CRATES_LIB:-$checkout/lib}
stage=$checkout/scratch/crates-test
crate_tool=$checkout/scripts/warp-crate
unset WARP_HOST WARP_RUST WARP_PIPELINE WARP_FALLBACK WARP_COMPILER WARP_SELF_HOSTED
export WARP_HOST_CACHE=$checkout/scratch/host-cache WARP_NO_WINDOW=1 WARP_HINTS=0

[ -x "$host" ] || { echo "✗ no warp-host at $host: cargo build --profile host -p warp-runtime --features host --bin warp-host"; exit 1; }
[ -f "$compiler_path" ] || { echo "✗ no compiler module at $compiler_path"; exit 1; }
command -v cargo >/dev/null || { echo "✗ no cargo: warp-crate builds the shims with it"; exit 1; }
rm -rf "$stage" && mkdir -p "$stage/bin" "$stage/lib/native" || exit 1
ln -f "$host" "$stage/bin/warp-host" 2>/dev/null || cp "$host" "$stage/bin/warp-host" || exit 1
cp "$compiler_path" "$stage/bin/compiler.wasm" || exit 1
host=$stage/bin/warp-host

run=0 failed=0
# expect <name> <exit status> <text the output holds>… [! <text it lacks>…] -- <command…>: stdout and stderr together
expect() {
	local name=$1 status=$2 wanted=() unwanted=() said got
	shift 2
	while [ "$1" != -- ]; do
		if [ "$1" = '!' ]; then unwanted+=("$2"); shift; else wanted+=("$1"); fi
		shift
	done
	shift
	said=$("$@" 2>&1)
	got=$?
	run=$((run + 1))
	local text problems=""
	for text in "${wanted[@]}"; do [[ $said == *"$text"* ]] || problems+=" missing [$text]"; done
	for text in "${unwanted[@]}"; do [[ $said == *"$text"* ]] && problems+=" unwanted [$text]"; done
	if [ "$got" = "$status" ] && [ -z "$problems" ]; then
		echo "✓ $name"
	else
		failed=$((failed + 1))
		echo "✗ $name: exit $got (expected $status),$problems"
		echo "$said" | sed 's/^/    /'
	fi
}

# the shims: built (or found in the cache) and installed
built=$("$crate_tool" build "$checkout"/lib/crates/{regex,sha2,crc32fast}.warp 2>&1)
status=$?
run=$((run + 1))
if [ $status = 0 ]; then echo "✓ warp-crate builds the regex, sha2 and crc32fast shims"; else
	failed=$((failed + 1))
	echo "✗ warp-crate build: exit $status"
	echo "$built" | sed 's/^/    /'
	expect "semver by scanned lines: a version requirement matches, a bad version's error" 0 "in range" "out of range" \
	"unexpected character 'x' while parsing major version number" -- runs semver.warp

echo "crates: $((run - failed)) passed / $run"
	exit 1
fi
# semver's lines drafted by rustdoc JSON alone (warp-crate scan), built as they come
"$crate_tool" scan semver@1 2>/dev/null | grep -v '^// skipped' >"$stage/semver.warp"
expect "warp-crate scan drafts semver's lines from rustdoc JSON" 0 "versionreq_matches(versionreq: handle, version: handle(semver::Version)): bool" \
	"version_free(version: handle(semver::Version)): void from 'rs_semver' = drop" -- cat "$stage/semver.warp"
expect "the scanned manifest builds unedited" 0 "librs_semver" -- "$crate_tool" build "$stage/semver.warp"
shims=$(dirname "$(echo "$built" | grep '/lib/librs_regex' | head -1)")
natives=$(dirname "$(echo "$built" | grep '/native/rs_regex.warp' | head -1)")
expect "a second build is the cache's" 0 "librs_sha2" ! "building" -- "$crate_tool" build "$checkout/lib/crates/sha2.warp"
printf '%s\n' 'crate: "regex@1"' "bad(x): i64 from 'rs_regex' = regex::escape" >"$stage/untyped.warp"
expect "a manifest parameter without a type is refused, naming it" 1 "the parameter 'x' of bad has no type" -- "$crate_tool" build "$stage/untyped.warp"
printf '%s\n' 'crate: "regex@1"' "bad(x: text): i64 from 'rs_regex' = regex::escape" "oops(x: text) from 'rs_regex' = regex::escape" >"$stage/broken.warp"
expect "a line that is almost a crate line is refused, not passed on" 1 "not a foreign line warp-crate reads" -- "$crate_tool" build "$stage/broken.warp"

# the lib folder the programs compile with: the checkout's, and the generated native files beside lib/native's
for entry in "$library"/*; do [ "$(basename "$entry")" = native ] || ln -s "$entry" "$stage/lib/"; done
[ -d "$library/native" ] && ln -s "$library"/native/* "$stage/lib/native/" 2>/dev/null
ln -sf "$natives"/rs_*.warp "$stage/lib/native/"
export WARP_LIB=$stage/lib WARP_LIBRARY_PATH=$shims LD_LIBRARY_PATH=$shims${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}
export DYLD_LIBRARY_PATH=$shims${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}
cd "$stage" || exit 1
program() { # program <name> <source lines…>: <name>.warp in the stage
	local name=$1
	shift
	printf '%s\n' "$@" >"$stage/$name.warp"
}
runs() { "$host" --compiler full run "$@"; }

program sha_line "sha256(data: text): text from 'rs_sha2'" 'sha256("abc")'
program sha_use 'use rs_sha2' 'print sha256("hello")' 'print sha512("")'
program crc 'use rs_crc32fast' 'crc32("abc")'
program regex_words 'use rs_regex' 'print(if is_match("hello world", "w.r") then "matches" else "no")' \
	'print(if is_match("hello", "^w") then "matches" else "no")' 'print first_match("a1 b22 c333", "[0-9]{2,}")' \
	'print replace_all("a1 b22", "[0-9]", "#")' 'print first_match("abc", "x") == ø'
program regex_all 'use rs_regex' 'found = find_all("a1 b22 c333 d4444", "[0-9]{2,}")' 'print #found' 'print found#3'
program regex_handle 'use rs_regex' 're = regex_new("b+")' 'print regex_find_start(re, "aabbb")' \
	'print regex_find_start(re, "xyz")' 'print regex_as_str(re)' 'regex_free(re)'
program semver 'use rs_semver' 'req = versionreq_parse(">=1.2, <2")' \
	'print(if versionreq_matches(req, version_parse("1.4.0")) then "in range" else "out")' \
	'print(if versionreq_matches(req, version_parse("2.0.1")) then "in range" else "out of range")' \
	'print version_parse("x.y")' 'print rs_semver_last_error()'
program regex_bad 'use rs_regex' 'is_match("x", "(")'
program regex_last_error 'use rs_regex' 're = regex_new("(")' 'print re' 'print rs_regex_last_error()'

expect "a foreign line calls Rust's sha2: sha256 hex" 0 "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad" -- runs sha_line.warp
expect "use rs_sha2: sha256 and sha512 words" 0 "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824" \
	"cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e" -- runs sha_use.warp
expect "use rs_crc32fast: crc32 of a text, its low byte the status" 194 "891568578" -- runs crc.warp
expect "regex words in warp over the shim: is_match, first_match, replace_all, ø" 0 "matches" "no" "22" "a# b##" "yes" -- runs regex_words.warp
expect "regex find_all: a list back through a list handle" 0 "3" "4444" -- runs regex_all.warp
expect "a regex handle: find_start, -1 for none, as_str, free" 0 "2" "-1" "b+" -- runs regex_handle.warp
expect "an invalid pattern raises the crate's error" 1 "regex parse error" -- runs regex_bad.warp
expect "a failed handle result is 0 and last_error says why" 0 "0" "unclosed group" -- runs regex_last_error.warp

expect "semver by scanned lines: a version requirement matches, a bad version's error" 0 "in range" "out of range" \
	"unexpected character 'x' while parsing major version number" -- runs semver.warp

echo "crates: $((run - failed)) passed / $run"
[ "$failed" = 0 ]
