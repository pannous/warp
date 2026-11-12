# the helpers of the shell tests that check what a warp run says (source it): check <what> <got> <expected> counts a
# failure, warning_of <code> is the warning lines a run of the code prints; the warp is WARP (default the warp on PATH)
warp=${WARP:-warp}
export WARP_WINDOW=0
failures=0
check() {
	if [ "$2" = "$3" ]; then echo "✓ $1"; else echo "✗ $1: got $2, expected $3"; failures=$((failures + 1)); fi
}
warning_of() { "$warp" --mute "$1" 2>&1 | grep '^warning' ; }
