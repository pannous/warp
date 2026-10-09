#!/bin/zsh
# Points Formula/warp.rb at release v<version>: its version and each prebuilt binary's sha256 (from the GitHub release
# the Release workflow filled), then copies it to the tap checkout if one exists. Usage: update_formula.sh 1.2.4
set -e
version=${1:?usage: update_formula.sh <version>}
formula=${0:A:h}/Formula/warp.rb
tap=$(brew --repository pannous/tap 2>/dev/null)/Formula/warp.rb
download=${0:A:h}/../../scratch/release-v$version

mkdir -p $download
gh release download v$version -R pannous/warp -p 'warp-*.tar.gz' -D $download --clobber
sed -i '' "s/^  version \".*\"/  version \"$version\"/" $formula
for archive in $download/warp-v$version-*.tar.gz; do
  target=${${archive:t}#warp-v$version-}; target=${target%.tar.gz}
  sum=$(shasum -a 256 $archive | cut -d' ' -f1)
  # the sha256 line right after this target's url line
  sed -i '' "/-$target\.tar\.gz\"/{n;s/sha256 \".*\"/sha256 \"$sum\"/;}" $formula
done
targets=$(grep -c 'url "#{RELEASE}' $formula)
archives=($download/warp-v$version-*.tar.gz)
(( ${#archives} == targets )) || { echo "release v$version has ${#archives} binaries, the formula wants $targets"; exit 1; }
[[ -f $tap ]] && cp $formula $tap && echo "copied to $tap"
