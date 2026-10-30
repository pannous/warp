#!/usr/bin/env bash
# scripts/prebuilt-parts.sh <folder>: every part of warp a user command would otherwise build on its first use built
# into the native warp's folder (an install, a release folder): today `warp build`'s driver in data/native. Then the
# first uses once more: exit 1, naming the files, when one still writes into the folder (user 2026-10-30: all modules
# and drivers prebuilt when installing; an installed /usr/lib/warp is read-only). after-push.sh runs it before the
# link switches, packaging/native_dist.sh before the tarball.
set -uo pipefail
HELLO=$(cd "$(dirname "$0")/.." && pwd)/samples/hello/hello.warp
folder=$(cd "${1:?usage: prebuilt-parts.sh <folder>}" && pwd) || exit 2
probe=$(mktemp -d "${TMPDIR:-/tmp}/prebuilt-parts.XXXXXX") || exit 1
trap 'rm -rf "$probe"' EXIT
cp "$HELLO" "$probe/" || exit 1
cd "$probe" || exit 1
unset WARP_LIB
export WARP_NO_WINDOW=1
timeout 600 "$folder/warp" build hello.warp >build.log 2>&1 ||
	{ echo "prebuilt-parts: warp build hello.warp failed in $folder: $(tail -1 build.log)" >&2; exit 1; }
touch before && sleep 1
{ timeout 300 "$folder/warp" build hello.warp; timeout 60 "$folder/warp" hello.warp; timeout 60 "$folder/warp" 'use math; 1'; } >/dev/null 2>&1
made=$(find "$folder" -type f -newer before | head -3)
[ -z "$made" ] || { echo "prebuilt-parts: a first use built a part of warp: ${made//$'\n'/, }" >&2; exit 1; }
