#!/bin/bash
# scripts/cloud-test.sh splits a filter argument with spaces into several filters (card cloud-test-lane: 5 runs of 0
# tests got "a b c" as one filter). Run with the Mac's /bin/bash 3.2: prints each argument list as [x][y]…
set -u
eval "$(sed '$d' "$(dirname "$0")/../scripts/cloud-test.sh")"   # its functions without the main call (bash 3.2 can't source <(…))
show() {
	local filters=() filter
	while IFS= read -r filter; do filters+=("$filter"); done < <(split_filters "${@:3}")
	set -- "$1" "$2" ${filters[@]+"${filters[@]}"}
	printf '[%s]' "$@"; echo
}
show branch tip "test_a test_b  test_c" test_d "parser::x::"   # [branch][tip][test_a][test_b][test_c][test_d][parser::x::]
show branch tip                                                  # [branch][tip]
show branch tip --test tests -- "p q"                            # [branch][tip][--test][tests][--][p][q]
