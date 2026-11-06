# hello's peak memory footprint (card peak-footprint-record; 2026-10-30: main grew hello's compile ~55 MiB in 66
# commits unnoticed). after-push records main's after each install (<store>/hello-footprint: MiB and commit), the smoke
# measures its branch's and complains loudly past FOOTPRINT_GROWTH_PERCENT over the record; it never fails a smoke.
# Sourced after platform.sh. macOS only (/usr/bin/time -l); elsewhere nothing is measured.
FOOTPRINT_GROWTH_PERCENT=10
footprint_record=$warp_store/hello-footprint

# footprint_run <MiB file> <seconds> <command…>: the command run for at most that long, its peak footprint in MiB
# written to the file
footprint_run() {
	local measured=$1 seconds=$2 timing status
	shift 2
	[ $warp_platform = mac ] || { timeout "$seconds" "$@"; return; }
	timing=$(mktemp "${TMPDIR:-/tmp}/footprint.XXXXXX")
	timeout "$seconds" /usr/bin/time -l -o "$timing" "$@"
	status=$?
	awk '/peak memory footprint/ { printf "%d\n", $1 / 1048576 }' "$timing" > "$measured"
	rm -f "$timing"
	return $status
}

# footprint_recorded <MiB> <commit>: the record of main's hello
footprint_recorded() { [ -n "$1" ] && echo "$1 $2" > "$footprint_record"; }

# footprint_checked <MiB> <what>: a loud line when the footprint grew past the record by more than the allowance
footprint_checked() {
	local now=$1 recorded commit
	[ -n "$now" ] && [ -f "$footprint_record" ] || return 0
	read -r recorded commit < "$footprint_record"
	(( now * 100 > recorded * (100 + FOOTPRINT_GROWTH_PERCENT) )) &&
		echo "FOOTPRINT: $2 peaks at $now MiB, main's hello at ${commit:0:10} at $recorded MiB (over +$FOOTPRINT_GROWTH_PERCENT %)" >&2
	return 0
}
