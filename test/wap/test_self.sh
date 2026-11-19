#!/usr/bin/env bash
# The self-build, file by file (notes/wap_review.md "The road to self-build", step 4): wap compiles one of its own
# files, the leaves of the use graph first, and the module it makes loads and runs. A file joins once wap compiles it
# as written and its functions give under wap what they give under warp. Source: warp-lead's self-build plan,
# 2026-11-18. Run it with bash test/wap/test_self.sh; the warp under test is WARP (default the warp on PATH)
source "$(dirname "$0")/compile.sh"
root="$(cd "$(dirname "$0")/../.." && pwd)"

# what loading the module wap makes of a file of wap's own says: "loads", or the compiler's or the run's error
loaded() {
	local module="$stage/$(basename "$1" .wap).wasm"
	$warp "$compiler" "$root/$1" "$module" 2>&1 && $warp "$module" >/dev/null 2>&1 && echo loads
}

# the files wap compiles alone and loads, leaves of the use graph first
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
)
for file in "${files[@]}"; do
	check "wap compiles $file and the result loads" "$(loaded "$file")" "loads"
done
exit $((failures > 0))
