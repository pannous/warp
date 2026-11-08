#!/usr/bin/env bash
# scripts/smoke.sh <worktree>: a branch's smoke test, building only what the branch needs (card lib-no-rebuild, user
# 2026-10-30: a library change needs no compiler rebuild). Hello runs with WARP_LIB=<worktree>/lib, which reads every
# changed module and sugar rule anew:
#   - lib/ only: the installed compilers are cloned into the worktree's lib/ (copy-on-write, ignored files) with the
#     inputs hash they were built of, then `scripts/rebuild.sh pure <worktree>` takes them only when that hash is the
#     tree's (card tree-installed: an install lagging main left modules of an older lib/, "markup.wasm won't link");
#     else the store's, or the stored compiler of this code with the runtime and modules built anew of this lib/
#   - core/, or a lib/ file the compiler reads at its own build (BAKED, compiler-inputs.sh --baked-files): said loudly
#     with the files, then `scripts/rebuild.sh pure <worktree>` (core/: run `scripts/self-check.sh <worktree>` too)
# after-push still builds the installed warp.wasm anew, so the lib it bundles is main's. Exit 0: hello printed.
set -uo pipefail
source "$(dirname "$0")/lib/platform.sh"
source "$(dirname "$0")/lib/hello-footprint.sh"
INSTALLED=$warp_binary
COMPILERS=(warp.wasm warp.wasm.inputs warp.wasm.cwasm warp.wasm.driver.cwasm warp.wasm.driver.cwasm.key warp.wasm.driver.build.cwasm warp.wasm.driver.build.cwasm.key runtime.wasm runtime.wasm.cwasm modules)
EXPECTED=Hello

tree=$(cd "${1:?usage: smoke.sh <worktree>}" && pwd) || exit 2
packages_linked "$tree"
scripts=$(cd "$(dirname "$0")" && pwd)
BAKED="^(core/|extensions/$(printf '|lib/%s$' $("$scripts/compiler-inputs.sh" --baked-files | sed 's/\./\\./g')))"
installed=$(dirname "$(readlink -f "$INSTALLED")")
base=$(git -C "$tree" merge-base origin/main HEAD) || exit 2
# a change writing code a second time in one file is refused before any build (card repeated-code-check)
"$scripts/repeated-code.sh" "$tree" || exit 1
changed=$({ git -C "$tree" diff --name-only "$base" HEAD; git -C "$tree" diff --name-only HEAD; } | sort -u)
baked=$(echo "$changed" | grep -E "$BAKED" | grep -v '^core/native/')  # the native warp is no compiler input (compiler-inputs.sh)
# whether it changes a top-level line of a lib/ module, one that may add, drop or rename a word other modules reach: a
# changed line at column 0 that is no comment (a body line is indented; a comment, sample or test edit changes none)
changes_module_words() {
	local modules=$(echo "$changed" | grep -E '^lib/(host/)?[A-Za-z_0-9]+\.warp$')
	[ -n "$modules" ] && git -C "$tree" diff -U0 "$base" -- $modules | grep -vE '^(\+\+\+|---) ' | grep -qE '^[-+][^[:space:]#]'
}
if [ -n "$baked" ]; then
	echo "smoke: REBUILDING, the compiler bakes in what this branch changed: $(echo $baked)"
else
	for part in "${COMPILERS[@]}"; do
		# machine code (.cwasm) is made at a module's first run (the 512 MB heap's at the compiler's first compile), so an
		# install may have none yet; the run makes it
		[[ $part == *.cwasm* && ! -e $installed/$part ]] && continue
		rm -rf "${tree:?}/lib/$part" && clone -R "$installed/$part" "$tree/lib/$part" || exit 1
	done
	for library in "$installed"/lib*warp_*.*; do clone "$library" "$tree/lib/" 2>/dev/null; done
fi
# a failed build prints its step, the step's output and the log's path in the store (and so does a hash failed before)
built=$("$scripts/rebuild.sh" pure "$tree" 2>&1) || { echo "smoke: rebuild failed:" >&2; echo "$built" >&2; exit 1; }
way=$(echo "$built" | tail -1)
# a lib/ change the compiler doesn't bake keeps the installed prebuilt modules, built of main's lib/: the standard
# modules are built anew of this lib/ and taken, so a change that breaks a module's prebuild fails here (card
# smoke-misses-prebuild: units.warp broke time.wasm past every smoke). All of them, in one run: a module reaches
# others by their words too (time reaches units in no use line and no signature), and one run is cheaper than a run
# per module, whose start dominates. Only for a change of a module's top-level lines (night decision
# smoke-prebuilds-modules: about 2.5-5.5 min more under load; a changed body still breaks only its own module's prebuild,
# which the smoke then misses)
if [ -z "$baked" ] && changes_module_words; then
	fresh=$tree/data/smoke-modules
	rm -rf "$fresh" && mkdir -p "$fresh" || exit 1
	made=$(cd "$tree" && WARP_LIB=$tree/lib WARP_NO_WINDOW=1 "$INSTALLED" "$tree/lib/warp.wasm" modules "$fresh" "$tree/lib/runtime.wasm" 2>&1) ||
		{ echo "smoke: the standard modules don't prebuild of this lib/:" >&2; echo "$made" | grep -v '^warning: ' | tail -5 >&2; exit 1; }
	for built in "$fresh"/*; do rm -f "$tree/lib/modules/${built##*/}" && cp "$built" "$tree/lib/modules/" || exit 1; done
	way="$way; modules prebuilt of this lib/"
fi
peak=$tree/data/smoke-hello-footprint
mkdir -p "$tree/data"
got=$(cd "$tree" && WARP_LIB=$tree/lib WARP_NO_WINDOW=1 footprint_run "$peak" 120 "$INSTALLED" samples/hello/hello.warp 2>&1 | without_runtime_noise | tail -1)
[[ $got == *$EXPECTED* ]] || { echo "smoke: hello got ${got:0:160}" >&2; exit 1; }
footprint_checked "$(cat "$peak" 2>/dev/null)" "this branch's hello"
# the native host module, which the driver links itself: an empty native_host.wasm failed every window, paint and
# sound run while hello passed (card empty-native-host)
host=$(cd "$tree" && WARP_LIB=$tree/lib WARP_NO_WINDOW=1 "$INSTALLED" 'window_possible()' 2>&1 | without_runtime_noise | tail -1)
[[ $host == no ]] || { echo "smoke: the native host module did not answer window_possible(): ${host:0:160}" >&2; exit 1; }
echo "smoke: $got ($way, WARP_LIB=$tree/lib)"
