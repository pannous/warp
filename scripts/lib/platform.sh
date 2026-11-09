# scripts/lib/platform.sh: what differs between the Mac and the Linux box, sourced by the scripts (bash and zsh) so no
# script branches on the platform itself. Defaults on the Mac are the Mac's paths as they were; env vars override.
case $(uname) in
Darwin)
	warp_platform=mac
	warp_library_extension=dylib warp_library_path_variable=DYLD_LIBRARY_PATH
	clone() { cp -c "$@"; }                # copy-on-write (APFS)
	sed_in_place() { sed -i '' "$@"; }
	mtime_of() { stat -f %m "$@"; }
	size_of() { stat -f %z "$@"; }
	;;
*)
	warp_platform=linux
	warp_library_extension=so warp_library_path_variable=LD_LIBRARY_PATH
	clone() { cp --reflink=auto "$@"; }    # a reflink where the file system has them (XFS, btrfs), else a copy
	sed_in_place() { sed -i "$@"; }
	mtime_of() { stat -c %Y "$@"; }
	size_of() { stat -c %s "$@"; }
	;;
esac
# the shared native warp: WARP_BINARY, else ~/dev/bin/warp (the Mac), else the warp on PATH (the box: ~/bin/warp)
warp_binary=${WARP_BINARY:-$HOME/dev/bin/warp}
# (neither there: the path stays, so an error names where the binary was looked for, never an empty one)
[ -e "$warp_binary" ] || warp_binary=$(command -v warp || echo "$warp_binary")
# the main checkout and the compiler store (scripts/rebuild.sh)
warp_main_checkout=${WARP_MAIN_CHECKOUT:-$HOME/dev/angles/warp}
# packages_linked <tree>: a worktree's packages/ the main checkout's, which git ignores, so a cowtree worktree has none
# and its package tests fail (card worktree-packages-link; scripts/link-packages.sh fills the main checkout's)
packages_linked() {
	local tree=$1
	[ -e "$tree/packages" ] || [ ! -d "$warp_main_checkout/packages" ] || [ "$tree" -ef "$warp_main_checkout" ] || ln -s "$warp_main_checkout/packages" "$tree/packages"
}
if [ $warp_platform = mac ]; then warp_store=${WARP_COMPILER_STORE:-/Volumes/Build/compilers}
elif [ -d /workspace/warp-store ]; then warp_store=${WARP_COMPILER_STORE:-/workspace/warp-store}  # the box's, shared by its agents
else warp_store=${WARP_COMPILER_STORE:-$HOME/.cache/warp/compilers}; fi
# a native warp copied into <folder> finds its wasmtime beside itself: put the folder on the loader's path (the Mac's
# rpath already looks there; Linux's loader needs LD_LIBRARY_PATH)
libraries_beside() { eval "export $warp_library_path_variable=\"$1\${$warp_library_path_variable:+:\$$warp_library_path_variable}\""; }
# the native warp's harmless stderr line under the wasm2c runtime (Linux), dropped before a run's last line is judged
without_runtime_noise() { grep -v '^wasm-rt warning'; }
# the SHA-256 of a file, 64 hex digits (sha256sum on Linux, shasum on the Mac)
sha256_of() { { sha256sum "$1" 2>/dev/null || shasum -a 256 "$1"; } | cut -c1-64; }
