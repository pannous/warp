#!/bin/bash
# Prints the body of each named test (folder::file::function) from tests/, first lines only
for name in "$@"; do
	IFS=: read -r folder _ file _ function <<< "$name"
	echo "== $name"
	awk -v fn="fn $function(" 'index($0, fn) { on = 1 } on { print; if (++n > 8 || /^}/) exit }' "tests/$folder/$file.rs"
done
