#!/usr/bin/env bash
# scripts/lib-smoke.sh <worktree>: the smoke test of a branch that changed lib/ only, with no rebuild (card
# lib-no-rebuild, user 2026-10-30: a library change needs no compiler rebuild). The installed compilers are cloned into
# the worktree's lib/ (copy-on-write, ignored files) and hello runs with WARP_LIB=<worktree>/lib, which reads every
# changed module and sugar rule anew. Refused (exit 2, naming the files) when the branch changed what the compiler bakes
# in when it is built: core/, or the lib/ files it reads at its own build time (BAKED: prelude, aliases, math's typed
# lines, rule words); those need `scripts/rebuild.sh pure <worktree>` (and core/: `scripts/self-check.sh`).
# after-push still builds the installed warp.wasm anew, so the lib it bundles is main's.
set -uo pipefail
INSTALLED=$HOME/dev/bin/warp
BAKED='^(core/|lib/prelude\.warp$|lib/aliases\.warp$|lib/math\.warp$|lib/rule_words\.warp$)'
COMPILERS=(warp.wasm warp.wasm.cwasm warp.wasm.driver.cwasm warp.wasm.driver.cwasm.key runtime.wasm runtime.wasm.cwasm modules)
EXPECTED=Hello

tree=$(cd "${1:?usage: lib-smoke.sh <worktree>}" && pwd) || exit 2
installed=$(dirname "$(readlink -f "$INSTALLED")")
base=$(git -C "$tree" merge-base origin/main HEAD) || exit 2
baked=$({ git -C "$tree" diff --name-only "$base" HEAD; git -C "$tree" diff --name-only HEAD; } | sort -u | grep -E "$BAKED")
if [ -n "$baked" ]; then
	echo "lib-smoke: the compiler bakes these in, rebuild instead (scripts/rebuild.sh pure $tree): $(echo $baked)" >&2
	exit 2
fi
for part in "${COMPILERS[@]}"; do
	rm -rf "${tree:?}/lib/$part" && cp -Rc "$installed/$part" "$tree/lib/$part" || exit 1
done
for library in "$installed"/lib*warp_*.*; do cp -c "$library" "$tree/lib/" 2>/dev/null; done
got=$(cd "$tree" && WARP_LIB=$tree/lib WARP_NO_WINDOW=1 timeout 120 "$INSTALLED" samples/hello/hello.warp 2>&1 | tail -1)
[[ $got == *$EXPECTED* ]] || { echo "lib-smoke: hello got ${got:0:160}" >&2; exit 1; }
echo "lib-smoke: $got (installed compilers, WARP_LIB=$tree/lib, no rebuild)"
