#!/usr/bin/env bash
# Points PKGBUILD at release v<version> (pkgver, each Linux binary's sha256, pkgrel 1), writes .SRCINFO from it and,
# with --push, commits both to the AUR repository warp-lang-bin (needs an ssh key registered on aur.archlinux.org).
# Runs on macOS and in the Release workflow. Usage: update_aur.sh 1.3.0 [--push]; notes/packaging.md
set -euo pipefail
version=${1:?usage: update_aur.sh <version> [--push]}
here=$(cd "$(dirname "$0")" && pwd)
download=$here/../../scratch/release-v$version
aur_repository=ssh://aur@aur.archlinux.org/warp-lang-bin.git

sha256=$(command -v sha256sum || echo "shasum -a 256") # Linux, macOS

mkdir -p "$download"
gh release download "v$version" -R pannous/warp -p '*-linux-gnu.tar.gz' -D "$download" --skip-existing
perl -pi -e "s/^pkgver=.*/pkgver=$version/; s/^pkgrel=.*/pkgrel=1/" "$here/PKGBUILD"
for arch in x86_64 aarch64; do
  sum=$($sha256 "$download/warp-v$version-$arch-unknown-linux-gnu.tar.gz" | cut -d' ' -f1)
  perl -pi -e "s/^sha256sums_$arch=.*/sha256sums_$arch=('$sum')/" "$here/PKGBUILD"
done

# what `makepkg --printsrcinfo` prints for this PKGBUILD's fields
write_srcinfo() {
  source "$here/PKGBUILD"
  field() { local key=$1; shift; for value in "$@"; do printf '\t%s = %s\n' "$key" "$value"; done; }
  echo "pkgbase = $pkgname"
  field pkgdesc "$pkgdesc"; field pkgver "$pkgver"; field pkgrel "$pkgrel"; field url "$url"
  field arch "${arch[@]}"; field license "${license[@]}"; field depends "${depends[@]}"
  field provides "${provides[@]}"; field conflicts "${conflicts[@]}"; field options "${options[@]}"
  for target in "${arch[@]}"; do
    local sources="source_$target[@]" sums="sha256sums_$target[@]"
    field "source_$target" "${!sources}"; field "sha256sums_$target" "${!sums}"
  done
  printf '\npkgname = %s\n' "$pkgname"
}
(write_srcinfo) > "$here/.SRCINFO"

if [[ ${2:-} == --push ]]; then
  checkout=$download/aur-warp-lang-bin
  rm -rf "$checkout" && git clone -q "$aur_repository" "$checkout"
  cp "$here/PKGBUILD" "$here/.SRCINFO" "$checkout/"
  git -C "$checkout" add PKGBUILD .SRCINFO
  git -C "$checkout" diff --cached --quiet || git -C "$checkout" commit -qm "warp-lang-bin $version"
  git -C "$checkout" push -q origin HEAD:master
  echo "pushed warp-lang-bin $version to the AUR"
fi
