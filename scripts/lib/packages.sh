# The registry packages' proxies (scripts/link-packages.sh, web/playground/build.sh): sourced, not run.
# A proxy's package.warp names a `repository` and a `version`; that tag is cloned once into
# ~/.cache/warp/packages/<name>@<version> (a tag never changes).
package_cache=${WARP_PACKAGES_CACHE:-$HOME/.cache/warp/packages}

# manifest_field <package.warp> <field>: its value, unquoted ("" for none)
manifest_field() { sed -n "s/^$2:[[:space:]]*//p" "$1" | head -1 | tr -d '"'"'" | sed 's/[[:space:]]*$//'; }

# package_clone <package.warp>: prints the folder of the proxy's pinned tag, cloned once; fails for a package that is
# no proxy or a clone that fails
package_clone() {
	local manifest=$1 name repository version clone
	name=$(basename "$(dirname "$manifest")")
	repository=$(manifest_field "$manifest" repository) version=$(manifest_field "$manifest" version)
	[[ -n $repository && -n $version ]] || return 1
	clone=$package_cache/$name@$version
	if [[ ! -d $clone/.git ]]; then
		echo "packages: fetching $repository v$version into $clone" >&2
		git clone -q --depth 1 --branch "v$version" "https://github.com/$repository.git" "$clone" 2>/dev/null ||
			{ echo "packages: cannot clone $repository v$version" >&2; return 1; }
	fi
	echo "$clone"
}
