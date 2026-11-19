# same-shape: function bodies and { } blocks of at least `least` tokens whose shapes match, names, numbers and texts
# erased. Prints `same-shape <file> <file:line> same shape as <file:line>` for each later copy. A table, data alone
# (names, numbers and texts between , : [ ]), has no logic to repeat and is no candidate
function keyword(word) { return word ~ /^(if|then|else|for|each|in|return|match|is|not|and|or|yes|no|ø|as|while|to|use|class|enum|raise|break|continue|_)$/ }
function add(kind, text) { n++; tokens[n] = kind == "N" && keyword(text) ? text : kind == "N" || kind == "0" || kind == "T" ? kind : text; files[n] = FILENAME; lines[n] = FNR
	if (text == ":=" && !definition) { definition = 1; body = n + 1 }
	if (text == "{") stack[++depth] = n
	else if (text == "}" && depth > 0) { candidate(stack[depth] + 1, n - 1); depth-- }
}
function shape(from, to,   i, s) { s = ""; for (i = from; i <= to; i++) s = s " " tokens[i]; return s }
function candidate(from, to) { if (to - from + 1 >= least && !data(from, to)) { count++; froms[count] = from; tos[count] = to } }
function data(from, to,   i) { for (i = from; i <= to; i++) if (tokens[i] !~ /^(N|0|T|,|:|\[|\])$/) return 0; return 1 }
FNR == 1 { if (definition) candidate(body, n); definition = 0; depth = 0 }
{
	line = $0; at = 1
	if (line ~ /^[^ \t#]/) { if (definition) candidate(body, n); definition = 0 }
	while (at <= length(line)) {
		c = substr(line, at, 1)
		if (c ~ /[ \t]/) { at++; continue }
		if (c == "#") break
		if (c == "\"") { j = at + 1; while (j <= length(line) && substr(line, j, 1) != "\"") j += substr(line, j, 1) == "\\" ? 2 : 1; add("T", ""); at = j + 1; continue }
		if (c == "'") { j = index(substr(line, at + 1), "'"); add("T", ""); at += j + 1; continue }
		if (match(substr(line, at), /^[0-9][0-9_.]*/)) { add("0", ""); at += RLENGTH; continue }
		if (match(substr(line, at), /^([A-Za-z_]|[\200-\377])([A-Za-z0-9_]|[\200-\377])*/)) { add("N", substr(line, at, RLENGTH)); at += RLENGTH; continue }
		if (match(substr(line, at), /^(:=|=>|->|==|!=|<=|>=|\?\?|\+=|-=|\.\.)/)) { add("P", substr(line, at, RLENGTH)); at += RLENGTH; continue }
		add("P", c); at++
	}
}
END {
	if (definition) candidate(body, n)
	# the longest first, so a copied function's copied blocks are not counted again
	for (i = 1; i <= count; i++) order[i] = i
	for (i = 2; i <= count; i++) { k = order[i]; j = i - 1; while (j > 0 && tos[order[j]] - froms[order[j]] < tos[k] - froms[k]) { order[j + 1] = order[j]; j-- } order[j + 1] = k }
	for (o = 1; o <= count; o++) {
		i = order[o]; from = froms[i]; to = tos[i]
		if (covered[from]) continue
		s = shape(from, to)
		if (s in first) {
			printf "same-shape\t%s\t%s:%d\tsame shape as %s\n", files[from], files[from], lines[from], first[s]
			for (t = from; t <= to; t++) covered[t] = 1
		} else first[s] = files[from] ":" lines[from]
	}
}
