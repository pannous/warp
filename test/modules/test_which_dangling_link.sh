#!/usr/bin/env bash
# which() and exists() see no program behind a dangling absolute link in the PATH (card which-missing-result): WASI
# refuses such a path (EPERM), which is no file there. Run it with bash test/modules/test_which_dangling_link.sh; the warp
# under test is WARP (default the warp on PATH), its compiler from WARP_LIB as usual
warp=${WARP:-warp}
stage=$(mktemp -d) || exit 1
trap 'rm -rf "$stage"' EXIT
ln -s /no-such-folder-warp-test "$stage/gone"
got=$(PATH="$stage/gone/bin:$PATH" "$warp" 'use shell; [which("warp_no_such_program") == ø, exists("'"$stage"'/gone/bin/x")]' 2>&1 | tail -1)
if [ "$got" = "[yes, no]" ]; then echo "✓ no program behind a dangling link"; else echo "✗ which/exists behind a dangling link: got $got, expected [yes, no]"; exit 1; fi
