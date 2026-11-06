#!/usr/bin/env bash
# Links the default registry's packages into a tree's packages/ folder, so programs and tests reach them by path,
# `use "packages/colors/colors.warp"` (decision package-use-by-path: the compiler looks nothing up by name).
# The registry github.com/pannous/warp-packages is cloned once to ~/.cache/warp/packages/.registry and pulled when
# older than an hour. A packages/<name> that is a real folder stays; a link is pointed at the registry's folder.
# A proxy (its package.warp names a `repository` and a `version`) has that tag cloned once into
# ~/.cache/warp/packages/<name>@<version> (a tag never changes); one whose `library: "dir/name"` has a native twin
# lib/native/<name>.c (uniscript's uniscript_c, card uniscript-ownership) gets that C library built of the clone, the
# crate in dir/ by cargo and the twin over it, into <clone>/native, and linked beside warp (where the native warp's
# dlopen looks first, as for libwarp_sound): `use uniscript` (lib/uniscript.warp) then calls it.
# usage: scripts/link-packages.sh [<tree>]   (default: this checkout)
set -u
registry_url=https://github.com/pannous/warp-packages.git
source "$(dirname "$0")/lib/packages.sh"
registry=$package_cache/.registry
tree=${1:-$(cd "$(dirname "$0")/.." && pwd)}
max_age_minutes=60

if [[ ! -d $registry/.git ]]; then
  git clone -q "$registry_url" "$registry" || { echo "link-packages: cannot clone $registry_url" >&2; exit 1; }
elif [[ ! -e $registry/.git/FETCH_HEAD || -n $(find "$registry/.git/FETCH_HEAD" -mmin +$max_age_minutes 2>/dev/null) ]]; then
  git -C "$registry" pull -q --ff-only || echo "link-packages: pull failed, using the clone as it is" >&2
fi
mkdir -p "$tree/packages"
for folder in "$registry"/*/; do
  folder=${folder%/}
  target=$tree/packages/$(basename "$folder")
  if [[ -d $target && ! -L $target ]]; then continue; fi
  ln -sfn "$folder" "$target"
done
echo "link-packages: $tree/packages linked to $registry"

# the folder of the warp binary on PATH, where the native warp looks for a library first
warp_folder() { local warp; warp=$(command -v warp) && dirname "$(readlink -f "$warp")"; }
case $(uname) in Darwin) extension=dylib origin=@loader_path ;; *) extension=so origin='$ORIGIN' ;; esac

# native_library <clone> <crate folder> <library name>: lib<name> built of the clone into <clone>/native, once
native_library() {
  local clone=$1 crate=$2 name=$3 out=$1/native built
  [[ -f $out/lib$name.$extension ]] && return 0
  echo "link-packages: building lib$name of $clone/$crate (cargo, once)"
  CARGO_TARGET_DIR=$clone/target nice -n 19 cargo build -q --release --manifest-path "$clone/$crate/Cargo.toml" || return 1
  mkdir -p "$out"
  built=$(ls "$clone"/target/release/lib*."$extension" | head -1)
  [[ -n $built ]] || return 1
  cp "$built" "$out/"
  cc -shared -fPIC -O2 -I"$clone/c" -I"$clone/$crate" "$tree/lib/native/$name.c" -L"$out" -l"$(basename "${built%.*}" | sed 's/^lib//')" \
    -Wl,-rpath,"$origin" -o "$out/lib$name.$extension"
}

for manifest in "$tree"/packages/*/package.warp; do
  [[ -f $manifest ]] || continue
  name=$(basename "$(dirname "$manifest")")
  version=$(manifest_field "$manifest" version)
  clone=$(package_clone "$manifest") || continue
  library=$(manifest_field "$manifest" library)
  [[ -n $library && -f $tree/lib/native/$(basename "$library").c ]] || continue
  library_name=$(basename "$library")
  native_library "$clone" "$(dirname "$library")" "$library_name" || { echo "link-packages: lib$library_name of $name did not build" >&2; continue; }
  beside=$(warp_folder) || continue
  for file in "$clone"/native/lib*."$extension"; do ln -sfn "$file" "$beside/$(basename "$file")"; done
  echo "link-packages: lib$library_name of $name v$version linked beside warp in $beside"
done
