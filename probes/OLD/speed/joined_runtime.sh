#!/bin/bash
# core/runtime.warp with the files its `use` lines name appended, as self_hosted.rs runtime_source joins it
# usage: probes/speed/joined_runtime.sh [lib folder]   (prints the joined source)
lib=${1:-$(dirname "$0")/../../lib}
joined="" used=""
while IFS= read -r line; do
	case "$line" in
		use\ \"*\") name=${line#use \"}; used+=$(cat "$lib/compiler/${name%\"}")$'\n'; joined+=$'\n' ;;
		use\ *) used+=$(cat "$lib/${line#use }.warp")$'\n'; joined+=$'\n' ;;
		*) joined+="$line"$'\n' ;;
	esac
done < "$lib/compiler/runtime.warp"
printf '%s%s' "$joined" "$used"
# card lowering-hangs (2026-10-21): `warp lower --json "$(probes/speed/joined_runtime.sh)"` takes ~2 s, also with
# `key_name(entry: any) := node_text(entry_key(entry))` above key_text and with instance_of moved before list_equal
# (the card's two repros, which used to spin for minutes)
