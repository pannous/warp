# Homebrew formula for the tap pannous/homebrew-tap (brew install pannous/tap/warp): the native warp of the tag's GitHub
# release, the folder holding it, its compilers, lib/ and libwarp_wasmtime in libexec (seconds, no build). Source of
# truth; the tap gets a copy.
# At a release: packaging/homebrew/update_formula.sh <version> writes the version and the binaries' sha256, see notes/packaging.md
class Warp < Formula
  desc "Wasm-first programming language and data notation, written in itself"
  homepage "https://github.com/pannous/warp"
  version "1.6.1"
  license all_of: ["AGPL-3.0-or-later", "MIT"]

  depends_on "binaryen" # wasm-opt, which `warp build` trims a program's module with

  RELEASE = "https://github.com/pannous/warp/releases/download/v#{version}/warp-v#{version}".freeze

  on_macos do
    on_arm do
      url "#{RELEASE}-aarch64-apple-darwin.tar.gz"
      sha256 "b51bcbd0a62b5f35759337eb8ae856e412d2d14453f66d30e996025d4594968d"
    end
    on_intel do
      url "#{RELEASE}-x86_64-apple-darwin.tar.gz"
      sha256 "dca7188c81f714a4a77f58119aff30ef0e08f096f0d493490becb0958e43c2fc"
    end
  end

  on_linux do
    on_arm do
      url "#{RELEASE}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "0e466294d8826523e7bac7eb4026546e1a8ba84f658c454950fad7a938b38091"
    end
    on_intel do
      url "#{RELEASE}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "7e049a1ee3953741621fd8eb521eb8b4fe7cb73ed400bc62422db8d13d8ba1f3"
    end
  end

  def install
    # the tarball is warp's lib/ with the native warp, its compilers and libwarp_wasmtime in it (decision
    # rust-gone-in-1-5): kept whole, bin/warp a link to its warp, which finds its lib/ and library beside its
    # real path
    libexec.install Dir["*"]
    bin.install_symlink libexec/"warp"
  end

  test do
    assert_equal "42", shell_output("#{bin}/warp '6*7'").strip.split.last
  end
end
