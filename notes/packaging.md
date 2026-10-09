# Packaging and releases

Decided 2026-10-06 (user: "publish under package managers with the highest priority, similarly to Wasp"). Wasp (C++)
ships through its own tap pannous/homebrew-wasp; warp follows the newer pattern of uniscript: the shared tap
pannous/homebrew-tap, formula built from source with cargo.

## Channels
- Homebrew: `brew install pannous/tap/warp`. Source of truth packaging/homebrew/Formula/warp.rb, copied to
  github.com/pannous/homebrew-tap/Formula/warp.rb. The fully qualified name matters: cloudflare/cloudflare/warp exists.
  Since v1.2.4 (user 2026-10-09: "publish a binary so it doesn't take ages to install") it installs the release's
  prebuilt binaries (7 s instead of a ~4 min cargo build); `brew install --HEAD pannous/tap/warp` still builds main
  from source (`brew reinstall` takes no --HEAD: uninstall first), `brew upgrade --fetch-HEAD` updates it.
- GitHub release per tag v<Cargo version>: .github/workflows/release.yml attaches
  warp-<tag>-<target>.tar.gz (warp + warp-runtime) for aarch64/x86_64-apple-darwin (Intel cross-built on the arm
  runner), x86_64/aarch64-unknown-linux-gnu.
- `cargo install --locked --git https://github.com/pannous/warp warp warp-runtime`.
- crates.io, prepared, not uploaded: packages `warp-lang` (user 2026-10-06; `warp` is the web framework; the library is
  still `warp`, the binary `warp`) and `warp-runtime` (1.2.3, same version). `cargo package --workspace` verifies both
  (227 files, 860 KB). Blocked by the postponed license (open_decisions P150): crates.io refuses a crate without
  `license`. Once decided: `license = "…"` in both Cargo.toml, a LICENSE file, then
  `CARGO_NET_OFFLINE=false cargo publish --workspace`, and the README's Cargo line becomes `cargo install warp-lang`
  (it installs warp; warp-runtime is a second `cargo install warp-runtime`).
- Every channel ships warp-runtime too: `warp <file>` builds the standalone executable from the warp-runtime next to
  warp (src/main.rs runtime_stub_path); without it warp tries to build it from its source checkout, gone after install.
  Built on its own (`-p warp-runtime`) it has no Cranelift.

## Verified 2026-10-06 (v1.2.3; the user set the version 1.2.3)
brew install/upgrade --build-from-source + brew test + brew style clean (~4 min build); the macOS release tarball
runs and builds a standalone executable; the README's `cargo install --git` works (run outside the repo: inside it,
.cargo/config.toml's `net.offline = true` applies and the git checkout is refused). That offline setting also broke
the first formula and release build: both set CARGO_NET_OFFLINE=false.

## At a release
1. Bump `version` in Cargo.toml, commit, push main.
2. `git tag v<version> && git push origin v<version>`: the Release workflow publishes the binaries (and Rust CI tests the tag).
3. When the Release run is green (all binaries uploaded): `packaging/homebrew/update_formula.sh <version>` writes the
   version and each binary's sha256 into the formula and copies it to the local tap; commit here, push the tap.
4. Test: `HOMEBREW_NO_AUTO_UPDATE=1 brew reinstall pannous/tap/warp && brew test pannous/tap/warp && brew style pannous/tap/warp`.
- Rust CI runs its tests on the tag (pushes to main skip them): on Linux it has failed since 2026-10-05 on 5 header
  tests (card ci-linux-headers); the release binaries do not depend on it.
