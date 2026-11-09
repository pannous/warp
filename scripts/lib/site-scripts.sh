# scripts/lib/site-scripts.sh: site_scripts_copied <checkout> <lib folder>, the playground's scripts a built site ships
# (every top-level web/playground/*.js and lib/libc.wasm, which extensions/commands/site.warp site_files picks from) into
# <lib folder>/site, where an installed warp or a release's `warp build --site` finds them (card build-site). Sourced by
# scripts/integrate/after-push.sh and packaging/native_dist.sh
site_scripts_copied() {
	local playground=$1/web/playground site=$2/site
	mkdir -p "$site/lib" &&
		cp "$playground"/*.js "$site/" &&
		cp "$playground/lib/libc.wasm" "$site/lib/"
}
