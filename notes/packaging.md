# Packaging and releases

warp is published under package managers like Wasp, through the shared tap pannous/homebrew-tap (uniscript's
pattern; Wasp (C++) has its own tap pannous/homebrew-wasp).

## Channels
- Homebrew: `brew install pannous/tap/warp`. Source of truth packaging/homebrew/Formula/warp.rb, copied to
  github.com/pannous/homebrew-tap/Formula/warp.rb. The fully qualified name matters: cloudflare/cloudflare/warp exists.
  It installs the release's prebuilt binaries (7 s instead of a ~4 min cargo build); `brew install --HEAD pannous/tap/warp` still builds main
  from source (`brew reinstall` takes no --HEAD: uninstall first), `brew upgrade --fetch-HEAD` updates it.
- GitHub release per tag v<Cargo version>: .github/workflows/release.yml attaches
  warp-<tag>-<target>.tar.gz (warp + warp-runtime) for aarch64/x86_64-apple-darwin (Intel cross-built on the arm
  runner), x86_64/aarch64-unknown-linux-gnu.
- PyPI, .deb/.rpm (license fields `AGPL-3.0-or-later AND MIT`: the compiler AGPL, warp-runtime MIT). Both repackage the release tarballs, jobs `packages`, `pypi`
  of release.yml after `binaries`; `gh workflow run release.yml -f tag=v<version> -f repackage=true` reruns only them
  from the existing tarballs (their sha256 stay, so the formula stays valid). Each one ships warp-runtime next to warp.
  - PyPI `pip install warp-language` (warp-lang is NVIDIA's Warp on PyPI):
    packaging/pypi/build_wheels.py writes one wheel per target, no Python inside, the binaries in
    `.data/scripts/` (maturin's bin layout; zip mode needs S_IFREG or pip drops the exec bit). Tags macosx_11_0_arm64,
    macosx_10_12_x86_64, manylinux_2_<glibc>_<arch> from the newest GLIBC_ symbol in the binary (2.39 today: the
    ubuntu-24.04 builders). Published by trusted publishing, no token, once the repository variable PYPI_PUBLISH=true.
  - `.deb`/`.rpm` warp-lang, from packaging/linux/nfpm.yaml (nfpm 2.47 in CI, `brew install nfpm` locally; run in the
    unpacked tarball, nfpm doesn't expand env vars in `src`): /usr/bin/warp, warp-runtime, warp-page-runtime, attached
    to the release as warp-lang_<v>_amd64.deb / warp-lang-<v>-1.x86_64.rpm (and arm64/aarch64).
  - No AUR : Arch users take the GitHub release's linux tarball.
  - Linux needs glibc >= 2.39 (Ubuntu 24.04, Debian 13, Fedora 40; not Ubuntu 22.04 or RHEL 9): the binaries are
    built on ubuntu-24.04 runners (card linux-glibc-floor).
  - One-time setup (user): PyPI account → pypi.org/manage/account/publishing "Add a new pending publisher": project
    warp-language, owner pannous, repository warp, workflow release.yml, environment pypi; then
    `gh variable set PYPI_PUBLISH -R pannous/warp -b true`. PyPI is live since v1.3.0.
- `cargo install --locked --git https://github.com/pannous/warp warp warp-runtime`.
- crates.io: `cargo install --locked warp-lang warp-runtime` (live since v1.3.0). Packages `warp-lang`
  (AGPL-3.0-or-later; `warp` is the web framework there; the library is still `warp`, the binary `warp`) and
  `warp-runtime` (MIT, same version). At a release: `CARGO_NET_OFFLINE=false cargo publish --workspace` (token in
  ~/.cargo/credentials.toml); Cargo.toml's `include` must list every file the sources include_str! (lib/, the
  playground's *.js), `cargo package --workspace` verifies.
- Every channel ships warp-runtime too: `warp <file>` builds the standalone executable from the warp-runtime next to
  warp (src/main.rs runtime_stub_path); without it warp tries to build it from its source checkout, gone after install.
  Built on its own (`-p warp-runtime`) it has no Cranelift.

## Gotcha
.cargo/config.toml's `net.offline = true` applies inside the repo: `cargo install --git` must run outside it, and the
formula and release build set CARGO_NET_OFFLINE=false.

## At a release
1. Bump `version` in Cargo.toml, commit, push main.
2. `git tag v<version> && git push origin v<version>`: the Release workflow builds the binaries, then (job
   `compilers`, by the linux binary from the tag's lib/) the warp-written compiler compiler.wasm and compiler-self.wasm
   (the bootstrap seed, user 2026-10-22; also release assets), then (job `tarballs`) one tarball per platform holding
   the binaries, both compilers and lib/ side by side; Rust CI tests the tag. An installed warp finds lib/ and the
   compilers next to its binary or in <prefix>/share/warp (Homebrew's pkgshare, deb/rpm's /usr/share/warp, the wheel's
   data) and never builds a compiler (card installed-compilers).
3. When the Release run is green (all binaries uploaded): `packaging/homebrew/update_formula.sh <version>` writes the
   version and each binary's sha256 into the formula and copies it to the local tap; commit here, push the tap.
4. Test: `HOMEBREW_NO_AUTO_UPDATE=1 brew reinstall pannous/tap/warp && brew test pannous/tap/warp && brew style pannous/tap/warp`,
   then `brew unlink warp`: a linked brew warp comes first on the user's PATH and shadows ~/dev/bin/warp (2026-10-22).
5. crates.io: `CARGO_NET_OFFLINE=false cargo publish --workspace` (outside the cloud lane; token in ~/.cargo/credentials.toml).
6. PyPI and deb/rpm need nothing: the tag's Release run publishes them.
- Rust CI runs its tests on the tag (pushes to main skip them); the release binaries do not depend on it.
