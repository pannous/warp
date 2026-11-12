#!/usr/bin/env bash
# a standard module states its own uses (a program's are implied): its call of another module's word without that use,
# plain or as a method, is the error `no function <word>`, its fix the use of each module defining the word, never
# "outside the core … (no owner)" (card unknown-method-owner).
# Run it with bash test/errors/test_module_word_unused.sh; the warp is WARP (default the warp on PATH), the lib whose
# compilers and modules it runs LIB (default the warp's own, beside its binary; a tree's: LIB=<tree>/lib)
warp=${WARP:-warp}
lib=${LIB:-$(dirname "$(readlink -f "$(command -v "$warp")")")}
export WARP_WINDOW=0
failures=0
# the lib under test with two probe modules beside its own, compiled from their source as any module not prebuilt
probe_lib=$(mktemp -d "${TMPDIR:-/tmp}/module-word-unused.XXXXXX")
trap 'rm -rf "$probe_lib"' EXIT
for entry in "$lib"/*; do ln -s "$entry" "$probe_lib/$(basename "$entry")"; done
printf 'cut(t) := t.remove_prefix("a")\n' >"$probe_lib/probe_text_word.warp"
printf 'once_each(xs) := unique(xs)\n' >"$probe_lib/probe_shared_word.warp"

# refused <code> <said…>: the run fails, says each part and no gap of the core
refused() {
	local code=$1 said part
	shift
	said=$("$warp" --lib "$probe_lib" "$code" 2>&1) && { echo "✗ $code ran: $said"; failures=$((failures + 1)); return; }
	for part in "$@"; do
		if [[ $said != *"$part"* || $said == *"outside the core"* ]]; then echo "✗ $code: $said"; failures=$((failures + 1)); return; fi
	done
	echo "✓ $code"
}

# a word of one module (text's remove_prefix), called as a method
refused 'use probe_text_word; cut("abc")' 'no function remove_prefix' 'use text'
# a word two modules define (shell's and list's unique): both named, never one guessed
refused 'use probe_shared_word; once_each([1, 1])' 'no function unique' 'use shell or use list'
[ $failures = 0 ]
