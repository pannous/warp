#!/bin/bash
# Builds web/uniscript: lib/uniscript.wasp compiled to uniscript.wasm by warp, the entity index of the uniscript package, and the OFL fonts as woff2.
# Usage: web/uniscript/build.sh [deploy]   (deploy: copy the page to the server afterwards)
set -euo pipefail

SERVER="pannous.com"
SERVER_DIR="/var/www/pannous/uniscript"
# only the OFL fonts: the Monaco/Menlo mirror fonts of fonts/dist are Apple fonts and must not be served
SERVED_FONTS="UniscriptSans-Regular.ttf UniscriptCJK-Regular.otf NewGardinerOmni2d4.ttf"
FONT_DIRS="fonts/dist $HOME/Library/Fonts"

page="$(cd "$(dirname "$0")" && pwd)"
repository="$(cd "$page/../.." && pwd)"
cd "$repository"

cargo build --offline --bin warp
"${CARGO_TARGET_DIR:-target}/debug/warp" compile lib/uniscript.wasp
mv lib/uniscript.wasm "$page/uniscript.wasm"

mkdir -p "$page/packages/uniscript/data" "$page/fonts"
cp packages/uniscript/data/entities.idx "$page/packages/uniscript/data/"  # fetched by the compile: lib/uniscript.wasp uses package uniscript

find_font() {
	for directory in $FONT_DIRS; do
		[ -f "$directory/$1" ] && { echo "$directory/$1"; return; }
	done
	echo "missing font $1: run python3 fonts/uniscript_fonts.py all" >&2
	exit 1
}

for font in $SERVED_FONTS; do
	woff2="$page/fonts/${font%.*}.woff2"
	source="$(find_font "$font")"
	[ "$woff2" -nt "$source" ] || python3 -c "
from fontTools.ttLib import TTFont
font = TTFont('$source'); font.flavor = 'woff2'; font.save('$woff2')"
done

if [ "${1:-}" = "deploy" ]; then
	ssh "$SERVER" "mkdir -p $SERVER_DIR"
	# no --delete: the directory also holds rust/, the uniscript repository's demo (its docs/make_demo.sh deploy)
	rsync -av --exclude build.sh --exclude .gitignore "$page/" "$SERVER:$SERVER_DIR/"
fi
