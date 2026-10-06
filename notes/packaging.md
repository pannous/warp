# Packaging and releases

Decided 2026-10-06 (user: "publish under package managers with the highest priority, similarly to Wasp"). Wasp (C++)
ships through its own tap pannous/homebrew-wasp; warp follows the newer pattern of uniscript: the shared tap
pannous/homebrew-tap, formula built from source with cargo.

## Channels
- Homebrew: `brew install pannous/tap/warp`. Source of truth packaging/homebrew/Formula/warp.rb, copied to
  github.com/pannous/homebrew-tap/Formula/warp.rb. The fully qualified name matters: cloudflare/cloudflare/warp exists.
- GitHub release per tag v<Cargo version>: .github/workflows/release.yml attaches
  warp-<tag>-<target>.tar.gz (warp + warp-runtime) for aarch64-apple-darwin, x86_64/aarch64-unknown-linux-gnu.
- `cargo install --locked --git https://github.com/pannous/warp warp warp-runtime`.
- Not on crates.io: the name `warp` belongs to the web framework (open decision: publish as another name?).
- Every channel ships warp-runtime too: `warp <file>` builds the standalone executable from the warp-runtime next to
  warp (src/main.rs runtime_stub_path); without it warp tries to build it from its source checkout, gone after install.
  Built on its own (`-p warp-runtime`) it has no Cranelift.

## At a release
1. Bump `version` in Cargo.toml, commit, push main.
2. `git tag v<version> && git push origin v<version>`: the Release workflow publishes the binaries (and Rust CI tests the tag).
3. `curl -sL https://github.com/pannous/warp/archive/refs/tags/v<version>.tar.gz | shasum -a 256` into the formula's
   url/sha256, commit here, copy to the tap and push it.
4. Test: `HOMEBREW_NO_AUTO_UPDATE=1 brew reinstall --build-from-source pannous/tap/warp && brew test pannous/tap/warp`.
