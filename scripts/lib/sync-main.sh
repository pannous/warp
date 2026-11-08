# scripts/lib/sync-main.sh, sourced: up_to_date_main [<checkout>] puts the checkout's own commits on top of origin/main,
# as `git pull --rebase --autostash` meant to, but by name: it fetches main alone, writes no FETCH_HEAD, and rebases
# onto origin/main. A pull rebases onto what FETCH_HEAD says, and the sessions sharing a checkout fetch at once and
# rewrite it under each other: "Cannot rebase onto multiple branches" (card pull-fetch-head-race). A merge commit stays
# a merge (--rebase-merges; a plain pull --rebase flattens it, notes/agents/handbook.md).
up_to_date_main() {
	local checkout=${1:-.}
	git -C "$checkout" fetch -q --no-write-fetch-head origin main && git -C "$checkout" rebase -q --autostash --rebase-merges origin/main
}
