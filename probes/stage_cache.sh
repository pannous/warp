#!/bin/zsh
# A compiled stage is kept for every identical lib/ (self_hosted.rs stage_key); it must run from the lib/ the run uses,
# not the one it was compiled in (card pure-lane-nondeterministic). Builds the stage from copy A, runs it from an
# identical copy B, deletes A, runs from B again: every run prints 3.
# usage: probes/stage_cache.sh [<lib>, default ./lib]
lib=${1:-$PWD/lib}
junk=/Volumes/Junk/stage-cache-$$
mark="# stage cache probe $$ $(date +%s)"
for copy in a b; do
  mkdir -p $junk/$copy && cp -R $lib $junk/$copy/lib && print -r -- $mark >> $junk/$copy/lib/compiler/walk.warp
done
run() { WARP_NO_WINDOW=1 WARP_HINTS=0 WARP_SELF_HOSTED=pure-only WARP_LIB=$1/lib timeout 300 /Users/me/dev/bin/warp 'print(1 + 2)' 2>&1 | tail -1 }
echo "built from a: $(run $junk/a)"
echo "run from b:   $(run $junk/b)"
rm -r $junk/a
echo "a deleted:    $(run $junk/b)"
rm -r $junk
