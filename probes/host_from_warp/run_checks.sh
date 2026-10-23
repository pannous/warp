#!/usr/bin/env bash
# run_checks.sh <warp-host> [--spawn] [<compiler-dir>]: tests/host/test_host.sh, unchanged, with warp-host replaced by
# driver.warp run by that warp-host (card host-from-warp, notes/host_from_warp_eval.md).
# - driver.warp is compiled once, by the given warp-host, to scratch/host-from-warp/driver.wasm
# - a shim `warp-host` (a shell script test_host.sh copies into its stage) runs `<warp-host> driver.wasm "$@"`
# - --spawn: the driver compiles and runs by warp-host processes (WARP_DRIVER_SPAWN), else in-process
# - <compiler-dir> holds compiler.wasm, compiler-self.wasm and a file `warp` (test_host.sh takes the compilers from
#   beside WARP_LAUNCHER_RUST); default: the folder of <warp-host>
# - the C modules' WAT has GC types: WARP_WAT2WASM, else wasm-tools on PATH (wabt 1.0.42's wat2wasm reads no `rec`)
# Prints test_host.sh's output: its ✓/✗ lines are the checks the driver passes or not.
set -u
checkout=$(cd "$(dirname "$0")/../.." && pwd)
real=$(cd "$(dirname "${1:?usage: run_checks.sh <warp-host> [--spawn] [<compiler-dir>]}")" && pwd)/$(basename "$1")
shift
spawn=""
[ "${1:-}" = --spawn ] && { spawn=$real; shift; }
compilers=${1:-$(dirname "$real")}
work=$checkout/scratch/host-from-warp
mkdir -p "$work"
export WARP_HOST_CACHE=$checkout/scratch/host-cache
"$real" compile "$checkout/probes/host_from_warp/driver.warp" "$work/driver.wasm" || { echo "✗ driver.warp does not compile"; exit 1; }
cat >"$work/warp-host" <<SHIM
#!/usr/bin/env bash
WARP_DRIVER_SPAWN="$spawn" exec "$real" "$work/driver.wasm" "\$@"
SHIM
chmod +x "$work/warp-host"
if [ -z "${WARP_WAT2WASM:-}" ] && command -v wasm-tools >/dev/null; then
	printf '#!/bin/sh\nexec wasm-tools parse "$1" -o "$2"\n' >"$work/wat-assembler" && chmod +x "$work/wat-assembler"
	export WARP_WAT2WASM=$work/wat-assembler
fi
[ -e "$compilers/warp" ] || { echo "✗ no $compilers/warp (test_host.sh takes the compilers from beside WARP_LAUNCHER_RUST)"; exit 1; }
WARP_HOST_BINARY=$work/warp-host WARP_LAUNCHER_RUST=$compilers/warp bash "$checkout/tests/host/test_host.sh"
