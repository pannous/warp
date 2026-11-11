#!/usr/bin/env bash
# scripts/smoke.sh <worktree>: a branch's smoke test, building only what the branch needs (card lib-no-rebuild, user
# 2026-10-30: a library change needs no compiler rebuild). Hello runs with --lib <worktree>/lib, which reads every
# changed module and sugar rule anew:
#   - lib/ only: the installed compilers are cloned into the worktree's lib/ (copy-on-write, ignored files) with the
#     inputs hash they were built of, then `scripts/rebuild.sh pure <worktree>` takes them only when that hash is the
#     tree's (card tree-installed: an install lagging main left modules of an older lib/, "markup.wasm won't link");
#     else the store's, or the stored compiler of this code with the runtime and modules built anew of this lib/
#   - core/ or extensions/, the compiler's own code (compiler-inputs.sh --code-files): said loudly
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
CODE_PATHS="^(core/|extensions/$(printf '|lib/%s$' $("$scripts/compiler-inputs.sh" --code-files | sed 's/\./\\./g')))"
installed=$(dirname "$(readlink -f "$INSTALLED")")
base=$(git -C "$tree" merge-base origin/main HEAD) || exit 2
# a change writing code a second time in one file is refused before any build (card repeated-code-check)
"$scripts/repeated-code.sh" "$tree" || exit 1
changed=$({ git -C "$tree" diff --name-only "$base" HEAD; git -C "$tree" diff --name-only HEAD; } | sort -u)
code_changed=$(echo "$changed" | grep -E "$CODE_PATHS" | grep -v '^core/native/')  # the native warp is no compiler input (compiler-inputs.sh)
# the lib/ modules the branch changed: the prebuilt lib/modules/*.wasm run in place of their source, so any edit of one
# (a body line too, card smoke-prebuild-body) needs them built anew before a probe sees it
changed_modules() { echo "$changed" | grep -E '^lib/(host/)?[A-Za-z_0-9]+\.warp$'; }
# a function a lib module defines anew at its top level that another lib file defines too: every call of the word may
# now reach the other definition (pattern-parameters' find(value: any, p: data) in text.warp made find("hello", "l")
# ø in every program, card smoke-lib-shadow; the choice itself is card lib-module). Said loudly with the test files
# that call the word, for the worker to run in the background: tests stay feedback (supervisor decision, option b)
clashes_said() {
	local module word others tests
	for module in $(changed_modules); do
		for word in $(git -C "$tree" diff -U0 "$base" -- "$module" | sed -nE 's/^\+(def )?([A-Za-z_][A-Za-z_0-9]*)\(.*/\2/p' | sort -u); do
			others=$(cd "$tree" && grep -lE "^(def )?$word\(" lib/*.warp lib/host/*.warp 2>/dev/null | grep -vx "$module")
			[ -z "$others" ] && continue
			tests=$(cd "$tree" && grep -rlE "(^|[^A-Za-z_0-9.])$word\(|\.$word([^A-Za-z_0-9]|$)" test --include='*.warp' | sort)
			echo "smoke: CLASH: $module defines $word, which $(echo $others) define(s) too: a call of $word may now reach the other one. Run in the background: warp test $(echo $tests)" >&2
		done
	done
}
if [ -n "$code_changed" ]; then
	echo "smoke: REBUILDING, the compiler code this branch changed: $(echo $code_changed)"
else
	for part in "${COMPILERS[@]}"; do
		# machine code (.cwasm) is made at a module's first run (the 512 MB heap's at the compiler's first compile), so an
		# install may have none yet; the run makes it
		[[ $part == *.cwasm* && ! -e $installed/$part ]] && continue
		rm -rf "${tree:?}/lib/$part" && clone -R "$installed/$part" "$tree/lib/$part" || exit 1
	done
	for library in "$installed"/lib*warp_*.* "$installed"/libwgpu_native.*; do clone "$library" "$tree/lib/" 2>/dev/null; done
fi
# a failed build prints its step, the step's output and the log's path in the store (and so does a hash failed before)
built=$("$scripts/rebuild.sh" pure "$tree" 2>&1) || { echo "smoke: rebuild failed:" >&2; echo "$built" >&2; exit 1; }
way=$(echo "$built" | grep -v "not prebuilt, compiled from source" | tail -1)
echo "$built" | grep "not prebuilt, compiled from source" >&2
# a lib/ change the compiler doesn't bake keeps the installed prebuilt modules, built of main's lib/: the standard
# modules are built anew of this lib/ and taken, so a change that breaks a module's prebuild fails here (card
# smoke-misses-prebuild: units.warp broke time.wasm past every smoke). All of them, in one run: a module reaches
# others by their words too (time reaches units in no use line and no signature), and one run is cheaper than a run
# per module, whose start dominates. For any change of a module's source (night decision smoke-prebuilds-modules:
# about 2.5-5.5 min more under load; card smoke-prebuild-body: a body-only edit left the stale module in place)
if [ -z "$code_changed" ] && [ -n "$(changed_modules)" ]; then
	fresh=$tree/data/smoke-modules
	rm -rf "$fresh" && mkdir -p "$fresh" || exit 1
	made=$(cd "$tree" && WARP_WINDOW=0 "$INSTALLED" --lib "$tree/lib" "$tree/lib/warp.wasm" modules "$fresh" "$tree/lib/runtime.wasm" 2>&1) ||
		{ echo "smoke: the standard modules don't prebuild of this lib/:" >&2; echo "$made" | grep -v '^warning: ' | tail -5 >&2; exit 1; }
	for built in "$fresh"/*; do rm -f "$tree/lib/modules/${built##*/}" && cp "$built" "$tree/lib/modules/" || exit 1; done
	way="$way; modules prebuilt of this lib/"
fi
peak=$tree/data/smoke-hello-footprint
mkdir -p "$tree/data"
got=$(cd "$tree" && WARP_WINDOW=0 footprint_run "$peak" 120 "$INSTALLED" --lib "$tree/lib" samples/hello/hello.warp 2>&1 | without_runtime_noise | tail -1)
[[ $got == *$EXPECTED* ]] || { echo "smoke: hello got ${got:0:160}" >&2; exit 1; }
footprint_checked "$(cat "$peak" 2>/dev/null)" "this branch's hello"
# the native host module, which the driver links itself: an empty native_host.wasm failed every window, paint and
# sound run while hello passed (card empty-native-host)
host=$(cd "$tree" && WARP_WINDOW=0 "$INSTALLED" --lib "$tree/lib" 'window_possible()' 2>&1 | without_runtime_noise | tail -1)
[[ $host == no ]] || { echo "smoke: the native host module did not answer window_possible(): ${host:0:160}" >&2; exit 1; }
clashes_said
echo "smoke: $got ($way, --lib $tree/lib)"
