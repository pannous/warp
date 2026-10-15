#!/bin/bash
# Builds web/uniscript: uniscript.warp (use uniscript: the uniscript package) compiled to uniscript.wasm by warp, the C library it imports
# (the package's uniscript_c.wasm) in modules/, and the OFL fonts in unicode-range slices.
# Usage: web/uniscript/build.sh [deploy]   (deploy: copy the page to the server afterwards)
set -euo pipefail

SERVER="pannous.com"
SERVER_DIR="/var/www/pannous/uniscript"
# slice_fonts.py serves only the OFL fonts: the Monaco/Menlo mirror fonts of the uniscript repository's fonts/dist are Apple fonts
FONT_DIRS="${UNISCRIPT_FONTS:-$HOME/dev/uniscript/fonts/dist} $HOME/Library/Fonts"

page="$(cd "$(dirname "$0")" && pwd)"
repository="$(cd "$page/../.." && pwd)"
cd "$repository"

warp="$(scripts/own-warp.sh)"
"$warp" compile --wasm web/uniscript/uniscript.warp  # → uniscript.wasm (compile alone builds an executable), fetches the package's library
# the modules it imports by path (`use uniscript`: the package's C library uniscript_c.wasm) beside the page as
# modules/<file>, where uniscript.js looks for them
mkdir -p "$page/modules" "$page/fonts"
wasm-tools print web/uniscript/uniscript.wasm | grep -oE '\(import "/[^"]+\.wasm"' | cut -d'"' -f2 | sort -u | while read -r module; do
	cp "$module" "$page/modules/"
	echo "shipped $module as modules/$(basename "$module")"
done
cp web/playground/imports.js "$page/"  # importDescriptors, shared with the playground

# fonts/: fonts.css with unicode-range slices of the fonts, fonts.json for sequence_fonts.js (needs fonttools, wordfreq)
python3 "$page/slice_fonts.py" "${FONT_DIRS// /:}" "$page/fonts"

if [ "${1:-}" = "deploy" ]; then
	ssh "$SERVER" "mkdir -p $SERVER_DIR"
	# no --delete: the directory also holds rust/, the uniscript repository's demo (its docs/make_demo.sh deploy)
	rsync -av --exclude build.sh --exclude .gitignore --exclude slice_fonts.py --exclude __pycache__ --exclude packages "$page/" "$SERVER:$SERVER_DIR/"
fi
