#!/usr/bin/env bash
# packaging/host_cache.sh <dir>: the shipped cache of warp-host (notes/warp_host.md) in <dir>/host-cache, for a dir laid
# out as a release tarball: warp-host, compiler-self.wasm and lib/ side by side. One run of hello computes
# compiler-self's kept values (~25 s); only the .kept files stay, ~0.4 MB and the same on every platform, as the key is
# compiler-self's bytes and lib/'s contents. The machine code (.cwasm, 18 MB, one platform's) is left out: a fresh
# install makes it on its first run (~4 s). Installed in <prefix>/share/warp/host-cache (or beside warp-host), where
# warp-host reads it after the user's own cache.
set -euo pipefail
dir=$(cd "${1:?usage: packaging/host_cache.sh <dir with warp-host, compiler-self.wasm and lib/>}" && pwd)
for file in warp-host compiler-self.wasm lib/compiler; do
	[ -e "$dir/$file" ] || { echo "host_cache.sh: no $file in $dir" >&2; exit 1; }
done
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
printf 'print "hello"\n' >"$work/hello.warp"
# as installed: lib/ and the compiler found beside warp-host, nothing of a checkout's or the user's
(cd "$dir" && env -u WARP_LIB -u WARP_HOST_SHIPPED_CACHE WARP_HOST_CACHE="$work/cache" ./warp-host run "$work/hello.warp") \
	| grep -qx hello || { echo "host_cache.sh: warp-host did not run hello" >&2; exit 1; }
rm -rf "$dir/host-cache" && mkdir -p "$dir/host-cache"
cp "$work"/cache/*.kept "$dir/host-cache/"
ls -l "$dir/host-cache"
