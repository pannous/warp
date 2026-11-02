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
[ -e "$warp_binary" ] || warp_binary=$(command -v warp)
# the main checkout and the compiler store (scripts/rebuild.sh)
warp_main_checkout=${WARP_MAIN_CHECKOUT:-$HOME/dev/angles/warp}
if [ $warp_platform = mac ]; then warp_store=${WARP_COMPILER_STORE:-/Volumes/Build/compilers}
elif [ -d /workspace/warp-store ]; then warp_store=${WARP_COMPILER_STORE:-/workspace/warp-store}  # the box's, shared by its agents
else warp_store=${WARP_COMPILER_STORE:-$HOME/.cache/warp/compilers}; fi
# do the compilers beside the shared binary follow main (the Mac's after-push refreshes them)? The box's are a
# release's, so a smoke there builds the branch's own (WARP_INSTALLED_FOLLOWS_MAIN=1 to clone them anyway)
if [ $warp_platform = mac ]; then warp_installed_follows_main=${WARP_INSTALLED_FOLLOWS_MAIN:-1}
else warp_installed_follows_main=${WARP_INSTALLED_FOLLOWS_MAIN:-0}; fi
# a native warp copied into <folder> finds its wasmtime beside itself: put the folder on the loader's path (the Mac's
# rpath already looks there; Linux's loader needs LD_LIBRARY_PATH)
libraries_beside() { eval "export $warp_library_path_variable=\"$1\${$warp_library_path_variable:+:\$$warp_library_path_variable}\""; }
# the native warp's harmless stderr line under the wasm2c runtime (Linux), dropped before a run's last line is judged
without_runtime_noise() { grep -v '^wasm-rt warning'; }
