#!/bin/bash
# Builds web/uniscript: uniscript.wasp (use uniscript: the uniscript package) compiled to uniscript.wasm by warp, the entity index of the uniscript package, and the OFL fonts in unicode-range slices.
# Usage: web/uniscript/build.sh [deploy]   (deploy: copy the page to the server afterwards)
set -euo pipefail

SERVER="pannous.com"
SERVER_DIR="/var/www/pannous/uniscript"
# slice_fonts.py serves only the OFL fonts: the Monaco/Menlo mirror fonts of fonts/dist are Apple fonts
FONT_DIRS="fonts/dist $HOME/Library/Fonts"

page="$(cd "$(dirname "$0")" && pwd)"
repository="$(cd "$page/../.." && pwd)"
cd "$repository"

cargo build --offline --bin warp
"${CARGO_TARGET_DIR:-target}/debug/warp" compile web/uniscript/uniscript.wasp  # → uniscript.wasm, fetches packages/uniscript

mkdir -p "$page/packages/uniscript/data" "$page/fonts"
cp packages/uniscript/data/entities.idx "$page/packages/uniscript/data/"  # fetched by the compile

# fonts/: fonts.css with unicode-range slices of the fonts, fonts.json for sequence_fonts.js (needs fonttools, wordfreq)
python3 "$page/slice_fonts.py" "${FONT_DIRS// /:}" "$page/fonts"

if [ "${1:-}" = "deploy" ]; then
	ssh "$SERVER" "mkdir -p $SERVER_DIR"
	# no --delete: the directory also holds rust/, the uniscript repository's demo (its docs/make_demo.sh deploy)
	rsync -av --exclude build.sh --exclude .gitignore --exclude slice_fonts.py --exclude __pycache__ "$page/" "$SERVER:$SERVER_DIR/"
fi
