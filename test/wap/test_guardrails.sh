#!/usr/bin/env bash
# wap's guardrails as checks (user decision via warp-lead, 2026-11-18), so they hold when nobody is looking:
#   one-fit          core/analyze has one type relation, `conversion`: no other function answers "does this type fit"
#                    (a yes/no or Conversion with a parameter `wanted`, two of given, held, these, those, or named same_type(s) or fits_type)
#   emit-infers-type core/emit computes no type: it reads the TYPE and BINDING facts typing and binding wrote and may
#                    choose by them, but calls no word of core/analyze and builds no type node (type_notation's builders)
#   dynamic-prim     a dynamic call is kernel typing writes: no `prim dispatched` anywhere, no `prim cast` in the preload
#   one-ranking      definitions are ordered only by `ranked` (analyze/calls.wap): no other sort in core/analyze or core/emit
#   long-file        no wap file over 300 lines: its owner splits it along a real seam
#   repeated-code    no line or list written twice in a file (scripts/repeated-code.sh)
#   same-shape       no function body or { } block of SHAPE_FLOOR tokens or more with the shape of another, names,
#                    numbers and texts erased (guardrail_shapes.awk): two such are one function or one table
#   boxed-any        `any` written as a type in wap's own source, outside preload/ (whose `any` is the language's
#                    dynamic type, the Node enum's Data among them): each spot a debt, the list empty before milestone 2
#                    (notes/decisions.md wap-boxed-any-debt); a place typing infers as any is ratchet_census.sh's
#   emit-boxes       only typing decides what is boxed (types.wap writes `prim box`): no struct.new of a box in core/emit
#                    outside that primitive; never allowlisted
#   one-owner        every wap file and test/wap script falls under exactly one row of notes/wap.md "Owners": a path or
#                    folder in its files column (wap/'s own paths written without it), or a row's "<folder> … no other
#                    row names"; a clause in words, a role row's "any file", owns nothing
#   in-tree          every wap file has its line in notes/wap.md's tree, the block of "## The tree": a file `<name>.wap`
#                    two spaces deeper than its folder's line `<folder>/`; never allowlisted
# Today's exceptions are in guardrails_allowed.txt, `<rule> <key> <count>`, key a function for one-fit and a file for
# the others. The list only shrinks: a count above it fails naming file:line and the rule, a count below it fails
# until the line is lowered or dropped. Run it with bash test/wap/test_guardrails.sh; it compiles nothing
RULES="one-fit emit-infers-type dynamic-prim one-ranking long-file repeated-code same-shape boxed-any emit-boxes one-owner in-tree"
# the rules no allowlist line excuses: a spot of theirs is a failure at once
UNLISTED="emit-boxes in-tree"
OWNERS=notes/wap.md
RELATION=conversion
LONGEST=300
SHAPE_FLOOR=20
# where a type is written: after `:`, `->`, `<`, `,` or `as`, or as an optional
ANY_TYPE='(:|->|<|,|[^a-z_]as)[ \t]*any([^a-z_?]|$)|[^a-z_.]any[?]'
# the words that build a type node (core/type_notation.wap); taking one apart is reading it
TYPE_BUILDERS="generic_type|type_built|function_type_of|optional_type|list_type|map_type|function_value_type"
SAME_TYPE_NAMES='^(same|fits)_types?$'
TYPE_PARAMETERS="given|held|these|those"

root="$(cd "$(dirname "$0")/../.." && pwd)"
allowed="$root/test/wap/guardrails_allowed.txt"
cd "$root" || exit 1
failures=0
wap_files=$(git ls-files 'wap/*.wap')

# grep -n's file:line:text as a violation of a rule keyed by its file
filed() { awk -F: -v rule="$1" '{ text = substr($0, length($1) + length($2) + 3); sub(/^[ \t]+/, "", text); printf "%s\t%s\t%s:%s\t%s\n", rule, $1, $1, $2, substr(text, 1, 100) }'; }
# each finder prints a violation a line: <rule> <key> <file:line> <text>, tab-separated
one_fit() {
	awk -v relation=$RELATION -v kinds="$TYPE_PARAMETERS" -v named="$SAME_TYPE_NAMES" '
	match($0, /^[a-z_]+\(.*\) as (bool|Conversion\??) /) {
		name = substr($0, 1, index($0, "(") - 1)
		parameters = substr($0, index($0, "(") + 1); sub(/\) as .*/, "", parameters)
		typed = 0; wanted = 0
		count = split(parameters, each, ",")
		for (i = 1; i <= count; i++) { gsub(/^ +|:.*/, "", each[i]); if (each[i] == "wanted") wanted = 1; if (each[i] ~ "^(" kinds ")$") typed++ }
		if (name != relation && (wanted || typed >= 2 || name ~ named)) printf "one-fit\t%s\t%s:%d\t%s\n", name, FILENAME, FNR, substr($0, 1, 100)
	}' wap/core/analyze/*.wap
}
# the functions core/analyze defines, a name the emitter's own files or the shared ones define too left out
analyze_words() {
	grep -ohE '^[a-z_]+\(' wap/core/analyze/*.wap | tr -d '(' | sort -u |
		grep -vxF -f <(grep -ohE '^[a-z_]+\(' wap/core/emit/*.wap wap/core/*.wap | tr -d '(' | sort -u)
}
emit_infers_type() {
	grep -nE "(^|[^a-z_.])($(analyze_words | paste -sd'|' -)|$TYPE_BUILDERS)\(" wap/core/emit/*.wap | filed emit-infers-type
}
dynamic_prim() {
	{ grep -nE 'prim dispatched\b' $wap_files; grep -nE 'prim cast\b' wap/preload/*.wap; } | filed dynamic-prim
}
one_ranking() {
	grep -nE 'sorted|min_by|max_by' wap/core/analyze/*.wap wap/core/emit/*.wap | grep -vE '^[^:]+:[0-9]+:ranked\(' | filed one-ranking
}
long_file() {
	wc -l $wap_files | awk -v longest=$LONGEST '$2 != "total" && $1 > longest { printf "long-file\t%s\t%s:%d\t%d lines\n", $2, $2, $1, $1 }'
}
repeated_code() {
	bash scripts/repeated-code.sh "$root" --list $wap_files | sed -E 's/^([^:]+):([0-9]+) (.*)/repeated-code\t\1\t\1:\2\t\3/'
}
same_shape() {
	LC_ALL=C awk -v least=$SHAPE_FLOOR -f test/wap/guardrail_shapes.awk $wap_files
}
boxed_any() {
	grep -nE "$ANY_TYPE" $(printf '%s\n' $wap_files | grep -v '^wap/preload/') | filed boxed-any | awk -F'\t' -v any="$ANY_TYPE" '{ code = $4; gsub(/"[^"]*"|#.*/, "", code) } code ~ any'
}
emit_boxes() {
	grep -nE 'STRUCT_NEW.*\bbox|\bbox.*STRUCT_NEW' wap/core/emit/*.wap | grep -v 'name == BOXED_VALUE' | filed emit-boxes
}
# the owner rows' files, `<row> <path>` a line, a folder ending in /, and `<row> * <folder>` for a catch-all
owned_paths() {
	awk -F'|' '/^\| stage \| owner/ { table = 1; next } table && !/^\|/ { exit } table && $2 !~ /^-+$/ {
		row++
		count = split($4, entries, /[,;]/)
		for (i = 1; i <= count; i++) {
			entry = entries[i]; gsub(/^ +| +$/, "", entry)
			if (entry ~ /no other row names/ && match(entry, /[^ ]+\//)) print row, "*", substr(entry, RSTART, RLENGTH)
			else if (entry != "" && entry !~ / /) print row, (entry ~ /^(test|scripts)\// ? "" : "wap/") entry
		}
	}' "$OWNERS"
}
one_owner() {
	git ls-files 'wap/*.wap' 'test/wap/*' | awk -v owners="$OWNERS" 'NR == FNR { if ($2 == "*") catch_all[$1] = $3; else path[NR] = $2; row[NR] = $1; next }
	{
		delete owning; count = 0
		for (i in path) if (($0 == path[i] || substr(path[i], length(path[i])) == "/" && index($0, path[i]) == 1) && !(row[i] in owning)) { owning[row[i]] = 1; count++ }
		if (count == 0) for (r in catch_all) if (index($0, catch_all[r]) == 1) count = 1
		if (count != 1) printf "one-owner\t%s\t%s:1\tunder %d rows of %s \"Owners\", not one\n", $0, $0, count, owners
	}' <(owned_paths) -
}
# the wap files notes/wap.md's tree places, a path a line: a line's indent says its depth, a folder line ending in /
tree_paths() {
	awk '/^## The tree/ { tree = 1 } tree && /^```/ { if (inside) exit; inside = 1; next }
	inside && NF {
		indent = match($0, /[^ ]/) - 1; depth = indent / 2
		if (depth > 6 || indent % 2) next
		path = ""; for (d = 0; d < depth; d++) path = path folder[d]
		if ($1 ~ /\/$/) folder[depth] = $1; else if ($1 ~ /\.wap$/) print path $1
	}' "$OWNERS"
}
in_tree() {
	printf '%s\n' $wap_files | grep -vxF -f <(tree_paths) | awk '{ printf "in-tree\t%s\t%s:1\tno line in the tree of notes/wap.md\n", $0, $0 }'
}

found=$(for rule in $RULES; do ${rule//-/_}; done)
# each rule and key, its count found and its count allowed
counts=$(printf '%s\n' "$found" | awk -F'\t' 'NF { print $1 " " $2 }' | sort | uniq -c | awk '{ print $2, $3, $1 }')
listed=$(grep -vE '^(#|$)' "$allowed")
for rule in $RULES; do
	said=$(join -a1 -a2 -e0 -o 0,1.2,2.2 <(printf '%s\n' "$counts" | awk -v r=$rule '$1 == r { print $2, $3 }' | sort) \
		<(printf '%s\n' "$listed" | awk -v r=$rule -v unlisted=" $UNLISTED " '$1 == r && !index(unlisted, " " r " ") { print $2, $3 }' | sort) |
		while read -r key count allows; do
			if [ "$count" -gt "$allows" ]; then
				printf '%s\n' "$found" | awk -F'\t' -v r=$rule -v k="$key" '$1 == r && $2 == k { print "  " $3 ": " $4 }'
				echo "  $rule: $key has $count, $allows allowed"
			elif [ "$count" -lt "$allows" ]; then
				echo "  $rule: $key has $count, the allowlist still says $allows: lower it in test/wap/guardrails_allowed.txt"
			fi
		done)
	case " $UNLISTED " in *" $rule "*) printf '%s\n' "$listed" | grep -q "^$rule " && said="$said
  $rule takes no allowlist lines: remove them from test/wap/guardrails_allowed.txt" ;; esac
	if [ -z "$said" ]; then echo "✓ $rule"; else printf '✗ %s\n%s\n' "$rule" "$said"; failures=$((failures + 1)); fi
done
exit $((failures > 0))
