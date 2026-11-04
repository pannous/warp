#!/usr/bin/env bash
# scripts/integrate/land.sh --tree <worktree>: step 2 of landing the branches of retired sessions, after merge.sh's
# tier 0 went green. Pushes exactly the tested hash to main, closes each merged card with its merge commit, deletes its
# remote branch; the push starts after-push.sh like any other (the pre-push hook, on-push.sh).
# --tree is required: a worker that ran land.sh bare from its own worktree landed the Integrator's tested batch
# (warp-web-host, 2026-11-02); workers push with plain git.
# A refused push (main moved) is tried again up to LAND_RETRIES times: when the commits pushed meanwhile leave the
# compiler's code alone (core/ and the files it bakes in, scripts/compiler-inputs.sh --baked-files) they are merged onto
# the tested hash as they are (a library change needs no rebuild, its pusher smoke-tested it; after-push checks hello on
# the result), else merge.sh merges and smoke-tests the batch anew, with no second self-check (the batch passed one, the
# new commits were tested by their pushers; night decision 00:50 2026-11-03: a 45 min self-check per retry never won). Main moves every few minutes with library pushes, so
# re-testing each of them kept a batch from ever landing (2026-11-02).
set -uo pipefail
LAND_RETRIES=${LAND_RETRIES:-8}

[ "${1-}" = --tree ] && [ -n "${2-}" ] || { echo "usage: scripts/integrate/land.sh --tree <worktree> (the Integrator's merge.sh tree; workers push with plain git)" >&2; exit 2; }
tree=$2
shift 2
tree=$(cd "$tree" && pwd) || exit 1
batch=$tree/data/integrate/batch
say() { echo "land: $*"; }
COMPILER_PATHS=(core extensions $("$(dirname "$0")/../compiler-inputs.sh" --baked-files | sed 's|^|lib/|'))
field() { sed -n "s/^$1 //p" "$batch"; }
set_field() { sed -i '' "s/^$1 .*/$1 $2/" "$batch"; }

# main moved past base: take its new commits onto the tested hash, re-testing only when they touch the compiler's paths
catch_up() {
	git -C "$tree" fetch -q origin || return 1
	if git -C "$tree" diff --quiet "$base" origin/main -- "${COMPILER_PATHS[@]}"; then
		git -C "$tree" merge -q --no-ff -m "Merge origin/main into the landing batch (no change to the compiler's code)" origin/main || return 1
		set_field tested "$(git -C "$tree" rev-parse HEAD)"
		set_field base "$(git -C "$tree" rev-parse origin/main)"
		say "main moved without compiler changes: merged, no new smoke test"
	else
		say "main moved with compiler changes: merge.sh again"
		"$(dirname "$0")/merge.sh" --tree "$tree" --smoke-only $(field card | cut -d' ' -f1)
	fi
}

tested=$(field tested) base=$(field base)
[ -n "$tested" ] || { say "no batch in $batch: run merge.sh first"; exit 1; }
[ "$(git -C "$tree" rev-parse HEAD)" = "$tested" ] || { say "the tree moved off the tested ${tested:0:10}"; exit 1; }
for ((attempt = 1; ; attempt++)); do
	git -C "$tree" push -q origin "$tested:refs/heads/main" && break
	((attempt < LAND_RETRIES)) || { say "push refused $LAND_RETRIES times: main keeps moving"; exit 1; }
	say "push refused (main moved), attempt $attempt of $LAND_RETRIES"
	catch_up || { say "RED after main moved: not landed"; exit 1; }
	tested=$(field tested) base=$(field base)
done
say "pushed ${tested:0:10} to main"

while read -r key branch ref; do
	git -C "$tree" merge-base --is-ancestor "$ref" "$tested" || { say "NOT MERGED $key ($ref)"; continue; }
	commit=$(git -C "$tree" log --merges --format=%H --grep="(card $key)" -1 "$base..$tested")
	todo done "$key" "$commit" >/dev/null && say "done $key"
	git -C "$tree" push -q origin --delete "$branch" 2>/dev/null
done < <(field card)
