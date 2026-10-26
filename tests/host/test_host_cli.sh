#!/usr/bin/env bash
# warp-host's command line as warp's (card warp-host-cli-parity): the values it prints, the words pure, self and
# version, --fuel and WARP_TIME_LIMIT ending a run that never stops, refusals in WARP_EMIT_REFUSALS. Results only:
# what a program prints, never an exit status or an error's wording.
# The host under test is WARP_HOST_BINARY (default the checkout's target/host/warp-host: `cargo build --profile host
# -p warp-runtime --features host --bin warp-host`), with the compiler pair from WARP_HOST_COMPILERS (default the
# checkout's data/, as scripts/rebuild.sh fills it from the store /workspace/cow/compilers), staged in
# scratch/host-cli-test/ as installed. Run it in a heavy slot: /workspace/box-slot.sh tests/host/test_host_cli.sh
checkout=$(cd "$(dirname "$0")/../.." && pwd)
host=${WARP_HOST_BINARY:-$checkout/target/host/warp-host}
compilers=${WARP_HOST_COMPILERS:-$checkout/data}
stage=$checkout/scratch/host-cli-test
export WARP_HOST_CACHE=$checkout/scratch/host-cache WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_LIB=$checkout/lib
unset WARP_TIME_LIMIT WARP_FUEL WARP_EMIT_REFUSALS

[ -x "$host" ] || { echo "✗ no warp-host at $host: cargo build --profile host -p warp-runtime --features host --bin warp-host"; exit 1; }
for compiler in warp.wasm warp-self.wasm; do
	[ -f "$compilers/$compiler" ] || { echo "✗ no $compiler in $compilers: scripts/rebuild.sh copies the pair"; exit 1; }
done
rm -rf "$stage" && mkdir -p "$stage/bin" || exit 1
cp "$host" "$stage/bin/warp-host" && cp "$compilers"/warp.wasm "$compilers"/warp-self.wasm "$stage/bin/" || exit 1
host=$stage/bin/warp-host
cd "$stage" || exit 1
program() { # program <name> <source lines…>: <name>.warp in the stage
	local name=$1
	shift
	printf '%s\n' "$@" >"$stage/$name.warp"
}

run=0 failed=0
# is <name> <printed> -- <command…>: the command's last line of output is <printed>
is() {
	local name=$1 wanted=$2 said
	shift 3
	said=$(timeout 300 "$@" 2>/dev/null | tail -1)
	run=$((run + 1))
	if [ "$said" = "$wanted" ]; then echo "✓ $name"; else failed=$((failed + 1)); echo "✗ $name: printed [$said], not [$wanted]"; fi
}
# holds <name> <text> -- <command…>: the command's output (stdout) holds <text>
holds() {
	local name=$1 wanted=$2 said
	shift 3
	said=$(timeout 300 "$@" 2>/dev/null)
	run=$((run + 1))
	if [[ $said == *"$wanted"* ]]; then echo "✓ $name"; else failed=$((failed + 1)); echo "✗ $name: printed [$said], lacks [$wanted]"; fi
}

# the values a run prints, as warp prints them
program hello '"Hello " + "🌍" + (2000 + 26)'
is "a text in double quotes" '"Hello 🌍2026"' -- "$host" hello.warp
is "a whole float without .0" 4 -- "$host" '4.0'
is "a float" 3.5 -- "$host" '3.5'
is "an exact sum" 0.3 -- "$host" '0.1 + 0.2'
is "an exact third" 1/3 -- "$host" '1/3'
is "an exact small fraction positionally" 0.00000095367431640625 -- "$host" '1/1048576'
is "a float from sqrt" 4 -- "$host" 'sqrt(16)'
is "a small float in scientific notation" 1.5e-7 -- "$host" '1.5e-7'
is "a float of 15 digits stays positional" 123456789012345 -- "$host" '123456789012345.0'
is "a bool" yes -- "$host" '1 < 2'
is "a one-letter text in a list in single quotes" "[1 'a' 'b' \"cd\"]" -- "$host" '[1 "a" '"'"'b'"'"' "cd"]'
is "a list with a bool and floats" '[yes 4 3.5]' -- "$host" '[true 4.0 3.5]'
program halved 'x = 7.0' 'x /= 2' 'x'
is "a float the program divided" 3.5 -- "$host" halved.warp
program record 'x = {name: "Al", age: 3}' 'x'
is "a map with bare keys" '{name:"Al" age:3}' -- "$host" record.warp
program quoted '"q\"uote"'
is "a quote in a text escaped" '"q\"uote"' -- "$host" quoted.warp
program raises 'raise "boom"'
is "a raised error is the run's printed value" 'Error("boom")' -- "$host" raises.warp

# the words of warp's commands
is "pure: warp.wasm" '"Hello 🌍2026"' -- "$host" pure hello.warp
is "self: warp-self.wasm" '"Hello 🌍2026"' -- "$host" self hello.warp
holds "version names warp.wasm" "warp.wasm of" -- "$host" version
holds "version names warp-self.wasm" "warp-self.wasm of" -- "$host" version

# limits: a run that never stops ends with an Error
program forever 'while true { 1 }'
is "--fuel n takes its steps, the program runs" '"Hello 🌍2026"' -- "$host" --fuel 100000000000 hello.warp
holds "--fuel ends a run that never stops" 'Error(' -- "$host" --fuel 1000000 forever.warp
holds "WARP_TIME_LIMIT ends a run that never stops" 'Error(' -- env WARP_TIME_LIMIT=1 "$host" forever.warp

# a refused program's refusal is a line of WARP_EMIT_REFUSALS
program refused 'x = eval("1+2")'
rm -f refusals.txt
WARP_EMIT_REFUSALS=$stage/refusals.txt "$host" refused.warp >/dev/null 2>&1
holds "a refusal goes to WARP_EMIT_REFUSALS" "outside the core: " -- cat refusals.txt

# WAT text, in a host built with the feature wat
printf '(module (func (export "main") (result i64) i64.const 7))\n' >seven.wat
is "a .wat module runs" 7 -- "$host" seven.wat

echo "$((run - failed)) of $run passed"
[ "$failed" = 0 ]
