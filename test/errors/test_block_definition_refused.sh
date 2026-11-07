#!/usr/bin/env bash
# A function defined inside an if, while or for block is refused with its fix (decision block-definitions, card
# inside): scoped functions are parked. A function's own body still defines. Run it with
# bash test/errors/test_block_definition_refused.sh; the warp under test is WARP (default the warp on PATH)
warp=${WARP:-warp}
export WARP_NO_WINDOW=1
failed=0
refused() {
	said=$("$warp" "$1" 2>&1 | tail -1)
	case $said in
		*"f is defined inside a block"*"f = y =>"*) echo "✓ refused: $1" ;;
		*) echo "✗ $1 expected the block-definition refusal, got: $said"; failed=1 ;;
	esac
}
refused 'i=0; while i<3 { i+=1; f(y) := i*y }; 0'
refused 'if yes { f(y) := y + 1; print f(2) }; 0'
refused 'for i in 1..2 { f := y => y + i }; 0'
said=$("$warp" 'g() := { f(y) := y + 1; f(2) }; g()' 2>&1 | tail -1)
[ "$said" = 3 ] && echo "✓ a function's own body defines: 3" || { echo "✗ g() expected 3, got: $said"; failed=1; }
exit $failed
