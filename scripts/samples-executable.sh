#!/bin/bash
# Makes samples runnable as `samples/x.warp`: prepends the warp shebang and sets the executable bit, in the
# working tree and in git's index. Arguments: .warp files (default: every tracked sample).
# Skipped: golf solutions (their byte count is the point) and @primitive files (built natively, not run).
SHEBANG='#!/usr/bin/env warp'
SKIP_PATH='^samples/golf/'
SKIP_FIRST_LINE='^@primitive'

cd "$(git rev-parse --show-toplevel)" || exit 1
[ $# -gt 0 ] || set -- $(git ls-files 'samples/' | grep '\.warp$')

for file in "$@"; do
    [[ "$file" == samples/*.warp && -f "$file" ]] || continue
    [[ "$file" =~ $SKIP_PATH ]] && continue
    first_line=$(head -n1 "$file")
    [[ "$first_line" =~ $SKIP_FIRST_LINE ]] && continue
    if [[ "$first_line" != '#!'* ]]; then
        { echo "$SHEBANG"; cat "$file"; } > "$file.shebang" && mv "$file.shebang" "$file"
        git add -- "$file"
    fi
    chmod a+x "$file"
    git update-index --chmod=+x -- "$file" 2>/dev/null
done
exit 0
