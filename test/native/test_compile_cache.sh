#!/usr/bin/env bash
# A program file's compile is cached (card test-suite-slowdown, decision by warp-supervisor 2026-11-18): run again,
# it runs the module kept from the first compile, while every file the compile read reads the same; a changed file
# compiles anew, never a stale run. Run it with bash test/native/test_compile_cache.sh; the warp is WARP (default on PATH)
source "$(dirname "$0")/checks.sh"
folder="$(cd "$(dirname "$0")/../.." && pwd)/data/compile-cache-test.$$"
mkdir -p "$folder" || exit 1
trap 'rm -rf "$folder"' EXIT
ran() { "$warp" --mute "$folder/$1" 2>&1; }

printf 'use "used.warp"\nprint answer()\n' >"$folder/main.warp"
printf 'answer() := 1\n' >"$folder/used.warp"
check "a first run" "$(ran main.warp)" "1"
check "the same program again" "$(ran main.warp)" "1"
printf 'answer() := 2\n' >"$folder/used.warp"
check "a used file changed: compiled anew" "$(ran main.warp)" "2"
printf 'answer() := 2 +\n' >"$folder/used.warp"
check "a compile that fails runs no earlier module" "$(ran main.warp | grep -c '^2$')" "0"

printf 'x = 7//2; x\n' >"$folder/warns.warp"
first=$(ran warns.warp | grep '^warning:')
check "a warning said" "$( [ -n "$first" ] && echo said)" "said"
check "the same warning said again from the cache" "$(ran warns.warp | grep '^warning:')" "$first"

# one cached module a program file: a compile by another compiler or lib/ (here a lib/ of its own, its entries linked)
# replaces it, never one more beside it, and the module of the other key is never run (card compile-cache)
mkdir "$folder/lib" && ln -s "$library"/* "$folder/lib/"
printf 'print 3\n' >"$folder/keys.warp"
check "compiled with one lib/" "$(ran keys.warp)" "3"
check "with lib/ by another path" "$("$warp" --mute --lib "$folder/lib" "$folder/keys.warp" 2>&1)" "3"
check "and the first again" "$(ran keys.warp)" "3"
check "one cached module of the file" "$(ls "$HOME/Library/Caches/warp/compiled" | grep "compile-cache-test.$$_keys.warp.*wasm$" | grep -vc "cwasm$")" "1"

# warp --no-cache compiles cold and keeps nothing (card cold-compile-option)
printf 'print 4\n' >"$folder/cold.warp"
check "a cold compile" "$("$warp" --mute --no-cache "$folder/cold.warp" 2>&1)" "4"
check "keeps no module" "$(ls "$HOME/Library/Caches/warp/compiled" | grep -c "compile-cache-test.$$_cold.warp")" "0"
exit $((failures > 0))
