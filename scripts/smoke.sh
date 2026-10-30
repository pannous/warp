#!/usr/bin/env bash
# scripts/smoke.sh <worktree>: a branch's smoke test, building only what the branch needs (card lib-no-rebuild, user
# 2026-10-30: a library change needs no compiler rebuild). Hello runs with WARP_LIB=<worktree>/lib, which reads every
# changed module and sugar rule anew:
#   - lib/ only: no build; the installed compilers are cloned into the worktree's lib/ (copy-on-write, ignored files)
#   - core/, or a lib/ file the compiler reads at its own build (BAKED, compiler-inputs.sh --baked-files): said loudly with the files, then `scripts/rebuild.sh pure <worktree>` first (core/: run
#     `scripts/self-check.sh <worktree>` too)
# after-push still builds the installed warp.wasm anew, so the lib it bundles is main's. Exit 0: hello printed.
set -uo pipefail
INSTALLED=$HOME/dev/bin/warp
COMPILERS=(warp.wasm warp.wasm.cwasm warp.wasm.driver.cwasm warp.wasm.driver.cwasm.key runtime.wasm runtime.wasm.cwasm modules)
EXPECTED=Hello

tree=$(cd "${1:?usage: smoke.sh <worktree>}" && pwd) || exit 2
scripts=$(cd "$(dirname "$0")" && pwd)
BAKED="^(core/$(printf '|lib/%s$' $("$scripts/compiler-inputs.sh" --baked-files | sed 's/\./\\./g')))"
installed=$(dirname "$(readlink -f "$INSTALLED")")
base=$(git -C "$tree" merge-base origin/main HEAD) || exit 2
baked=$({ git -C "$tree" diff --name-only "$base" HEAD; git -C "$tree" diff --name-only HEAD; } | sort -u | grep -E "$BAKED")
if [ -n "$baked" ]; then
	echo "smoke: REBUILDING, the compiler bakes in what this branch changed: $(echo $baked)"
	built=$("$scripts/rebuild.sh" pure "$tree" 2>&1) || { echo "smoke: rebuild failed: $(echo "$built" | tail -2)" >&2; exit 1; }
	echo "$built" | tail -1
	way="its own build"
else
	for part in "${COMPILERS[@]}"; do
		rm -rf "${tree:?}/lib/$part" && cp -Rc "$installed/$part" "$tree/lib/$part" || exit 1
	done
	for library in "$installed"/lib*warp_*.*; do cp -c "$library" "$tree/lib/" 2>/dev/null; done
	way="the installed compilers, no rebuild"
fi
got=$(cd "$tree" && WARP_LIB=$tree/lib WARP_NO_WINDOW=1 timeout 120 "$INSTALLED" samples/hello/hello.warp 2>&1 | tail -1)
[[ $got == *$EXPECTED* ]] || { echo "smoke: hello got ${got:0:160}" >&2; exit 1; }
echo "smoke: $got ($way, WARP_LIB=$tree/lib)"
