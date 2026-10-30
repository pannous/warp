#!/usr/bin/env bash
# seed-tags-to-sources.sh: each seed release's tag moved onto the commit its seed was built from (the commit its notes
# name); the tag keeps its name, so the release and its asset stay. Run once by hand: it force-pushes tags
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
moved() { git tag -f "$1" "$2" >/dev/null && git push -q --force origin "refs/tags/$1" && echo "$1 -> $2"; }
moved seed-2026-10-30-5433ac43 6047e4f431
moved seed-2026-10-30-e7ae6aaf 8e6d06d275
moved seed-2026-10-29-bd1ab597 4470f018e5
moved seed-2026-10-29-5e25bfb0 c4d710d64a
moved seed-2026-10-29-7afa4465 64a443271f
moved seed-2026-10-29-f7c80528 0b2df6e915
moved seed-2026-10-29-c44fef73 4f359ef6cf
moved seed-2026-10-28-c4c33d67 eb868fd16e
moved seed-2026-10-28-d181ba4d 95e5146b61
moved seed-2026-10-28-05e046bb 23f9fda52d
moved seed-2026-10-28-f0b7f82e de78100a40
moved seed-2026-10-28-2922a99f 153cdc2685
moved seed-2026-10-28-1a04836c c0ed03fb02
moved seed-2026-10-28-a088516c 49abe704ad
moved seed-2026-10-28-19d92f8c 5795c640a6
moved seed-2026-10-28-aa94bc35 0677c02689
moved seed-2026-10-27-18e7a6e7 041a4b87a1
moved seed-2026-10-27-1ec0b599 8b4d2a2521
moved seed-2026-10-27-5ed07ab1 636fd3c681
moved seed-2026-10-27-e59091d8 6d7ab78ee2
moved seed-2026-10-27-637f66c1 f633ebd243
moved seed-2026-10-27-5b6ab5a4 6dbcef045b
moved seed-2026-10-27-14fb03b1 b2c84b4d36
moved seed-2026-10-27-bd332e14 38b8cfc4d5
moved seed-2026-10-30-3efd5391 e47a024c37
