#!/bin/zsh
# The build tweak of a worktree: every package of the workspace gets a "-<suffix>" version, so its artifacts in the
# shared target dir are not swapped with another checkout's build of the same package and version
# (notes/build_speed.md "copies share one warp artifact"). Uncommitted, never part of a branch.
#   version-tweak.sh <suffix> [worktree]          apply (worktree: default the current directory)
#   version-tweak.sh restore [worktree]           put the committed manifests and Cargo.lock back (before a merge,
#                                                  a checkout or a push)
suffix=${1:?usage: version-tweak.sh <suffix>|restore [worktree]}
cd ${2:-.} || exit 1
manifests=(Cargo.toml crates/*/Cargo.toml(N))
if [[ $suffix == restore ]]; then
	for file in $manifests Cargo.lock; do git cat-file -e HEAD:$file 2>/dev/null && git show HEAD:$file > $file; done
	exit 0
fi
for file in $manifests; do
	sed -i '' "/^\\[package\\]\$/,/^version = /{/-$suffix\"\$/!s/^version = \"\\([^\"]*\\)\"\$/version = \"\\1-$suffix\"/;}" $file
done
# a path dependency's version requirement ("1.2.3", for crates.io) must accept the tweaked pre-release version
sed -i '' "s/\\(path = \"crates\\/[^\"]*\", version = \"\\)\\([^\"-]*\\)\"/\\1\\2-$suffix\"/" Cargo.toml
/usr/bin/grep -H '^version' $manifests
