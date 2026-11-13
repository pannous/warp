# Packaging and releases

warp is published under package managers like Wasp, through the shared tap pannous/homebrew-tap (uniscript's
pattern; Wasp (C++) has its own tap pannous/homebrew-wasp).

## Channels
- Homebrew: `brew install pannous/tap/warp`. Source of truth packaging/homebrew/Formula/warp.rb, copied to
  github.com/pannous/homebrew-tap/Formula/warp.rb. The fully qualified name matters: cloudflare/cloudflare/warp exists.
  It installs the release's prebuilt binaries (no --HEAD build).
- GitHub release per tag v<VERSION>: .github/workflows/release.yml attaches warp-<tag>-<target>.tar.gz (the native
  warp, its compilers, lib/ and libwarp_wasmtime, packaging/native_dist.sh) for aarch64/x86_64-apple-darwin and
  x86_64/aarch64-unknown-linux-gnu.
- Windows (card windows-build, no archive yet): release.yml's `windows` job compiles the native warp on windows-latest
  from the C the compilers job writes (`warp build native --c core/native/warp.warp`: no warp runs on Windows to build
  it there) with llvm-mingw's clang, against our wasmtime C API built there (warp_wasmtime.dll), and runs hello and
  smoke's checks. lib/native/windows.c answers the POSIX calls (lib/native/system.h includes it, the one place the C
  asks the system): paths with forward slashes and no drive letter on the current drive, whose root the modules see
  as /. Still missing: fork, popen's shell (`warp test`, `warp build`), other drives (cards windows-native-modules,
  crlf-line-endings).
- PyPI, .deb/.rpm (license fields `AGPL-3.0-or-later AND MIT`: the compiler AGPL, warp-runtime MIT). Both repackage the release tarballs, jobs `packages`, `pypi`
  of release.yml after `binaries`; `gh workflow run release.yml -f tag=v<version> -f repackage=true` reruns only them
  from the existing tarballs (their sha256 stay, so the formula stays valid).
  - PyPI `pip install warp-language` (warp-lang is NVIDIA's Warp on PyPI):
    packaging/pypi/build_wheels.py writes one wheel per target, its only Python the pure package warp_language/ (`import warp_language.node`: warp data text <-> Python values, no binary needed; test/tools/test_warp_node.py; named warp_language, not warp, which warp-lang's NVIDIA Warp owns): the release folder whole in
    `.data/data/share/warp/` (modes kept; zip mode needs S_IFREG or pip drops the exec bit) and a launcher
    `.data/scripts/warp` running its warp, which reads its lib/ as its own folder (card packaging-pypi). Tags
    macosx_11_0_arm64, macosx_10_12_x86_64, manylinux_2_<glibc>_<arch> from the newest GLIBC_ symbol in the native warp
    (2.35 today: the ubuntu-22.04 builders). Published by trusted publishing, no token, once the repository variable PYPI_PUBLISH=true.
  - `.deb`/`.rpm` warp-lang, from packaging/linux/nfpm.yaml (nfpm 2.47 in CI, `brew install nfpm` locally; run in the
    unpacked tarball, nfpm doesn't expand env vars in `src`): the tarball as /usr/lib/warp, /usr/bin/warp a link, attached
    to the release as warp-lang_<v>_amd64.deb / warp-lang-<v>-1.x86_64.rpm (and arm64/aarch64).
  - No AUR : Arch users take the GitHub release's linux tarball.
  - Linux needs glibc >= 2.35 (Ubuntu 22.04, Debian 12, Fedora 36): the binaries are built on ubuntu-22.04 runners
    and release.yml checks each against GLIBC_FLOOR (card linux-glibc-floor). warp and libwarp_wasmtime ask 2.34
    (RHEL 9), libwarp_gpu 2.35 (hypot). Lower (manylinux_2_28) needs an older container; the bootstrap, the previous
    release's warp, would then have to ask no newer glibc than that container either.
  - One-time setup: PyPI account → pypi.org/manage/account/publishing "Add a new pending publisher": project
    warp-language, owner pannous, repository warp, workflow release.yml, environment pypi; then
    `gh variable set PYPI_PUBLISH -R pannous/warp -b true`. PyPI is live since v1.3.0.

## At a release
1. Bump `VERSION` in extensions/commands/commands.warp, write notes/releases/v<version>.md, commit, push main.
   Then a dry run of that commit, green before the tag: `gh workflow run release.yml -f tag=$(git rev-parse <commit>) -f dry=true` (the full SHA: checkout takes a short one for a branch name). It alone
   compiles lib/native's C on the release's own runners (ubuntu-22.04 x64 and arm, the old glibc and gcc the tarballs
   must run on); pages.yml's bootstrap compiles it only on ubuntu-latest (v1.8.0's Linux jobs failed on what 24.04
   accepted: RTLD_DEFAULT undeclared, card release-linux-bootstrap).
2. `git tag v<version> && git push origin v<version>`: the Release workflow (.github/workflows/release.yml) makes a
   draft release, builds the compilers from the tag's lib/ by the newest seed (job `compilers`), then one tarball per
   platform (job `native`: the native warp, the compilers and lib/ side by side), then the .deb and .rpm. Each job's
   warp is the previous release's native warp rebuilt from the tag's core/native first (scripts/release-bootstrap.sh,
   run from the workflow's own commit, so an existing tag gets a fixed bootstrap: `gh workflow run release.yml -f
   tag=v<version> [-f dry=true]`, never a moved tag). An installed warp finds lib/ and the compilers next to its binary
   or in <prefix>/share/warp and never builds a compiler (card installed-compilers).
3. When the Release run is green (all binaries uploaded): `packaging/homebrew/update_formula.sh <version>` writes the
   version and each binary's sha256 into the formula and copies it to the local tap; commit here, push the tap.
4. Test: `HOMEBREW_NO_AUTO_UPDATE=1 brew reinstall pannous/tap/warp && brew test pannous/tap/warp && brew style pannous/tap/warp`,
   then `brew unlink warp`: a linked brew warp comes first on the user's PATH and shadows ~/dev/bin/warp.
5. PyPI and deb/rpm need nothing: the tag's Release run publishes them.
