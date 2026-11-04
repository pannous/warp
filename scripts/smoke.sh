#!/usr/bin/env bash
# scripts/smoke.sh <worktree>: a branch's smoke test, building only what the branch needs (card lib-no-rebuild, user
# 2026-10-30: a library change needs no compiler rebuild). Hello runs with WARP_LIB=<worktree>/lib, which reads every
# changed module and sugar rule anew:
#   - lib/ only: no build; the installed compilers are cloned into the worktree's lib/ (copy-on-write, ignored files),
#     and a changed lib/*.warp has the standard modules prebuilt anew by them, failing loudly as a build would
#   - core/, or a lib/ file the compiler reads at its own build (BAKED, compiler-inputs.sh --baked-files): said loudly with the files, then `scripts/rebuild.sh pure <worktree>` first (core/: run
#     `scripts/self-check.sh <worktree>` too)
# after-push still builds the installed warp.wasm anew, so the lib it bundles is main's. Exit 0: hello printed.
set -uo pipefail
source "$(dirname "$0")/lib/platform.sh"
INSTALLED=$warp_binary
COMPILERS=(warp.wasm warp.wasm.cwasm warp.wasm.driver.cwasm warp.wasm.driver.cwasm.key warp.wasm.driver.build.cwasm warp.wasm.driver.build.cwasm.key runtime.wasm runtime.wasm.cwasm modules)
EXPECTED=Hello

tree=$(cd "${1:?usage: smoke.sh <worktree>}" && pwd) || exit 2
scripts=$(cd "$(dirname "$0")" && pwd)
BAKED="^(core/|extensions/$(printf '|lib/%s$' $("$scripts/compiler-inputs.sh" --baked-files | sed 's/\./\\./g')))"
installed=$(dirname "$(readlink -f "$INSTALLED")")
base=$(git -C "$tree" merge-base origin/main HEAD) || exit 2
baked=$({ git -C "$tree" diff --name-only "$base" HEAD; git -C "$tree" diff --name-only HEAD; } | sort -u | grep -E "$BAKED" | grep -v '^core/native/')  # the native warp is no compiler input (compiler-inputs.sh)
if [ -n "$baked" ] || [ "$warp_installed_follows_main" != 1 ]; then
	if [ -n "$baked" ]; then echo "smoke: REBUILDING, the compiler bakes in what this branch changed: $(echo $baked)"
	else echo "smoke: REBUILDING, the installed compilers do not follow main (WARP_INSTALLED_FOLLOWS_MAIN)"; fi
	# a failed build prints its step, the step's output and the log's path in the store (and so does a hash failed before)
	built=$("$scripts/rebuild.sh" pure "$tree" 2>&1) || { echo "smoke: rebuild failed:" >&2; echo "$built" >&2; exit 1; }
	echo "$built" | tail -1
	way="its own build"
else
	for part in "${COMPILERS[@]}"; do
		# machine code (.cwasm) is made at a module's first run (the 512 MB heap's at the compiler's first compile), so an
		# install may have none yet; the run makes it
		[[ $part == *.cwasm* && ! -e $installed/$part ]] && continue
		rm -rf "${tree:?}/lib/$part" && clone -R "$installed/$part" "$tree/lib/$part" || exit 1
	done
	for library in "$installed"/lib*warp_*.*; do clone "$library" "$tree/lib/" 2>/dev/null; done
	way="the installed compilers, no rebuild"
	# a changed module is prebuilt anew by the installed compiler, as a build's modules step does: one that does not
	# prebuild fails the smoke (card scripts-smoke, 2026-11-04: a lib-only `: list` broke every build unseen)
	if { git -C "$tree" diff --name-only "$base" HEAD; git -C "$tree" diff --name-only HEAD; } | grep -qE '^lib/.*\.warp$'; then
		prebuilt=$(cd "$tree" && WARP_LIB=$tree/lib WARP_NO_WINDOW=1 "$INSTALLED" "$tree/lib/warp.wasm" modules "$tree/lib/modules" "$tree/lib/runtime.wasm" 2>&1) ||
			{ echo "smoke: build-failed at step modules:" >&2; echo "$prebuilt" | without_runtime_noise | tail -3 >&2; exit 1; }
		way="$way, the changed lib's modules prebuilt"
	fi
fi
got=$(cd "$tree" && WARP_LIB=$tree/lib WARP_NO_WINDOW=1 timeout 120 "$INSTALLED" samples/hello/hello.warp 2>&1 | without_runtime_noise | tail -1)
[[ $got == *$EXPECTED* ]] || { echo "smoke: hello got ${got:0:160}" >&2; exit 1; }
echo "smoke: $got ($way, WARP_LIB=$tree/lib)"
