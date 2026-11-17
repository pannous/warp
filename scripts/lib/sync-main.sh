# scripts/lib/sync-main.sh, sourced: up_to_date_main [<checkout>] puts the checkout's own commits on top of origin/main,
# as `git pull --rebase` meant to, but by name: it fetches main alone, writes no FETCH_HEAD. A pull rebases onto what
# FETCH_HEAD says, and the sessions sharing a checkout fetch at once and rewrite it under each other: "Cannot rebase
# onto multiple branches" (card pull-fetch-head-race).
# Nothing is stashed (the user's rule; the stash list is shared by all worktrees), yet the user's main checkout usually
# holds uncommitted edits: `git replay` rebases the own commits without touching the files, and `reset --keep` moves
# the checkout there, carrying every edit along and refusing loudly when an incoming change touches an edited file.
# Own merge commits, which replay can't carry, go through `rebase --rebase-merges` (a plain pull --rebase flattens
# them, notes/agents/handbook.md), which refuses a dirty tree loudly.
# Another session sharing the checkout may switch branch or commit while this works out the target: the reset then
# would land on its branch and drop its commit (card sync-main-race), so branch and HEAD are pinned at the start and
# checked again right before each move.
checkout_state() { git -C "$1" symbolic-ref -q HEAD; git -C "$1" rev-parse HEAD; }
refuse_when_moved() {
	[ "$(checkout_state "$1")" = "$2" ] && return
	echo "up_to_date_main: $1 moved (branch or HEAD changed by another session) since it started: nothing reset, run it again" >&2
	return 1
}
up_to_date_main() {
	local checkout=${1:-.} replayed target=origin/main started
	git -C "$checkout" fetch -q --no-write-fetch-head origin main || return
	started=$(checkout_state "$checkout")
	if [ -n "$(git -C "$checkout" rev-list --merges -1 origin/main..HEAD)" ]; then
		refuse_when_moved "$checkout" "$started" || return
		git -C "$checkout" rebase -q --rebase-merges origin/main
		return
	fi
	# prints "update <ref> <new> <old>" when the checkout has commits of its own, nothing when it has none
	replayed=$(git -C "$checkout" replay --ref-action=print --onto origin/main origin/main..HEAD) || return
	[ -n "$replayed" ] && target=$(echo "$replayed" | awk '{ print $3 }')
	refuse_when_moved "$checkout" "$started" || return
	git -C "$checkout" reset -q --keep "$target"
}
