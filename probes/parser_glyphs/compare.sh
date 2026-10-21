#!/bin/bash
# Each line of codes.txt (code from test_glyph_aliases, test_emoji_atoms, test_math_operators) run through the front
# lane (parser.warp, WARP_SELF_HOSTED=front) and the Rust front end: prints the lines whose values differ
# (card parser-glyph-aliases). WARP_LIB: the lib/ under test
cd "$(dirname "$0")"
export WARP_NO_WINDOW=1 WARP_HINTS=0
differ=0
while IFS= read -r code; do
	rust=$(warp "$code" 2>&1 | tail -1)
	front=$(WARP_SELF_HOSTED=front warp "$code" 2>&1 | tail -1)
	if [ "$rust" != "$front" ]; then
		echo "$code    rust: $rust    front: $front"
		differ=$((differ + 1))
	fi
done < codes.txt
echo "$differ of $(wc -l < codes.txt) differ"
