#!/usr/bin/env bash
# An error after a uniscript entity names the column the source writes it at: `\:alpha` is read as α, but `nope`
# stays at column 22 of `α = 1; x = \:alpha + nope` (card parser-cold-forms, the Interviewer's condition on the
# entities source pass). Run it with bash test/errors/test_entity_error_column.sh; the warp under test is WARP (default
# the warp on PATH), its compiler from WARP_LIB as usual
warp=${WARP:-warp}
export WARP_NO_WINDOW=1
said=$("$warp" 'α = 1; x = \:alpha + nope' 2>&1 | tail -1)
case $said in
	*"nope at 1:22"*) echo "✓ the error keeps its written column: $said" ;;
	*) echo "✗ expected nope at 1:22, got: $said"; exit 1 ;;
esac
