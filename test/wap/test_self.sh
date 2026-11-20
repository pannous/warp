#!/usr/bin/env bash
# The self-build, file by file (notes/wap_review.md "The road to self-build", step 4): wap compiles one of its own
# files, the leaves of the use graph first, and the module it makes loads and runs. A file joins once wap compiles it
# as written and its functions give under wap what they give under warp. Source: warp-lead's self-build plan,
# 2026-11-18. Run it with bash test/wap/test_self.sh; the warp under test is WARP (default the warp on PATH)
source "$(dirname "$0")/compile.sh"
root="$(cd "$(dirname "$0")/../.." && pwd)"
# how long wap1 may take to compile wap's compiler (milestone 2, notes/backbone.md)
SELF_COMPILE_SECONDS=60

# what loading the module wap makes of a file of wap's own says: "loads", or the compiler's or the run's error
loaded() {
	local module="$stage/$(basename "$1" .wap).wasm"
	$warp "$compiler" "$root/$1" "$module" 2>&1 && $warp "$module" >/dev/null 2>&1 && echo loads
}

# the files wap compiles alone and loads, in the order each became loadable
files=(
	wap/core/emit/opcodes.wap
	wap/kernel/forms.wap
	wap/core/emit/wasm.wap
	wap/core/stage.wap
	wap/core/errors.wap
	wap/core/parse/lexemes.wap
	wap/core/analyze/definers.wap
	wap/core/bindings.wap
	wap/core/paths.wap
	wap/core/type_notation.wap
	wap/preload/operators.wap
	wap/core/emit/emitter.wap
	wap/core/emit/equality.wap
	wap/core/emit/function_values.wap
	wap/core/emit/functions.wap
	wap/core/emit/host.wap
	wap/core/emit/lists.wap
	wap/core/emit/patterns.wap
	wap/core/emit/places.wap
	wap/core/emit/primitives.wap
	wap/core/emit/values.wap
	wap/core/parse/atoms.wap
	wap/core/parse/cascade.wap
	wap/core/parse/layout.wap
	wap/core/parse/reader.wap
	wap/core/analyze/names.wap
	wap/preload/lists.wap
	wap/preload/numbers.wap
	wap/preload/rules.wap
	wap/preload/sugar/calls.wap
	wap/preload/sugar/conditions.wap
	wap/preload/sugar/data.wap
	wap/preload/sugar/definitions.wap
	wap/preload/sugar/literals.wap
	wap/preload/sugar/loops.wap
	wap/preload/sugar/matching.wap
	wap/preload/sugar/ranges.wap
	wap/preload/sugar/records.wap
	wap/preload/sugar/types.wap
	wap/core/format.wap
	wap/core/language.wap
	wap/core/parse/operator_table.wap
	wap/core/parse/operators.wap
	wap/core/parse/tokens.wap
	wap/core/rules/lines.wap
	wap/core/rules/lowering.wap
	wap/core/rules/matcher.wap
	wap/core/rules/patterns.wap
	wap/core/rules/stages.wap
	wap/core/rules/templates.wap
	wap/core/rules/typed.wap
	wap/core/analyze/calls.wap
	wap/core/analyze/conditions.wap
	wap/core/analyze/data.wap
	wap/core/analyze/declarations.wap
	wap/core/analyze/dispatch.wap
	wap/core/analyze/fits.wap
	wap/core/analyze/function_values.wap
	wap/core/analyze/letters.wap
	wap/core/analyze/matching.wap
	wap/core/analyze/modules.wap
	wap/core/analyze/scope.wap
	wap/core/analyze/shapes.wap
	wap/core/analyze/types.wap
	wap/core/analyze/variables.wap
	wap/core/pipeline.wap
)
for file in "${files[@]}"; do
	check "wap compiles $file and the result loads" "$(loaded "$file")" "loads"
done
# Stage 2: the module wap makes of its own compiler, wap1, compiles a program, and that program runs
wap1=$(compiler_built "$stage/self" 1)
# wap1 built, or why not
wap1_built() { [ -f "$wap1" ] || { echo "$wap1"; return 1; }; }
self_built_hello() {
	wap1_built || return
	printf 'print "hello from wap"\n' >"$stage/hello.wap"
	$warp "$wap1" "$stage/hello.wap" "$stage/hello.wasm" 2>&1 && $warp "$stage/hello.wasm" 2>&1
}
check "the compiler wap makes of itself compiles a program that runs" "$(self_built_hello)" "hello from wap"
# and compiles wap's compiler in under a minute (milestone 2, notes/backbone.md)
self_compile_time() {
	wap1_built || return
	local started=$(date +%s) seconds
	$warp "$wap1" "$source_compiler" "$stage/self/wap2.wasm" 2>&1 || return
	seconds=$(($(date +%s) - started))
	[ $seconds -lt $SELF_COMPILE_SECONDS ] && echo "under a minute" || echo "$seconds s"
}
check "wap1 compiles wap's compiler in under a minute" "$(self_compile_time)" "under a minute"
exit $((failures > 0))
