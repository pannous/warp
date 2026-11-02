# scripts/lib/results-header.sh: the one header line every tracked result file of status lines starts with
# (test/samples.txt, test/samples-web.txt, test/tests-web.txt; card nightly-mac-samples):
#   # <commit> <date> <time> <passed>/<total> <percent>%
# Readers of these files skip lines starting with "#".
RESULTS_HEADER_MARK='# '

# results_header <file> <commit> <passing status regex>: (re)writes the file's header from its status lines
results_header() {
	local file=$1 commit=$2 passing=$3 body passed total
	body=$(grep -av "^$RESULTS_HEADER_MARK" "$file")
	total=$(grep -ac . <<<"$body")
	passed=$(grep -acE "^($passing) " <<<"$body")
	{
		awk -v commit="$commit" -v when="$(date '+%Y-%m-%d %H:%M')" -v passed="$passed" -v total="$total" -v mark="$RESULTS_HEADER_MARK" \
			'BEGIN { printf "%s%s %s %d/%d %.1f%%\n", mark, commit, when, passed, total, total ? 100 * passed / total : 0 }'
		echo "$body"
	} >"$file.headed" && mv "$file.headed" "$file"
}

# results_rate <file>: the percent its header names, empty without a header
results_rate() { head -1 "$1" 2>/dev/null | sed -n "s/^$RESULTS_HEADER_MARK.* \([0-9.]*\)%$/\1/p"; }
