# Homebrew formula for the tap pannous/homebrew-tap (brew install pannous/tap/warp): the native warp of the tag's
# GitHub release, the folder holding it, its compilers, lib/ and libwarp_wasmtime in libexec (seconds, no build).
# Source of truth; the tap gets a copy.
# At a release: packaging/homebrew/update_formula.sh <version> writes the version and the binaries' sha256,
# see notes/packaging.md
class Warp < Formula
  desc "Wasm-first programming language and data notation, written in itself"
  homepage "https://github.com/pannous/warp"
  version "1.14.0"
  license all_of: ["AGPL-3.0-or-later", "MIT"]

  depends_on "binaryen" # wasm-opt, which `warp build` trims a program's module with

  RELEASE = "https://github.com/pannous/warp/releases/download/v#{version}/warp-v#{version}".freeze

  on_macos do
    on_arm do
      url "#{RELEASE}-aarch64-apple-darwin.tar.gz"
      sha256 "57c814a797cf9ad896a475723079038a573d88ab76b2fda44a3ccdb6296e762f"
    end
    on_intel do
      url "#{RELEASE}-x86_64-apple-darwin.tar.gz"
      sha256 "3a896ece25970008799cb8edb53ce953db17d84d0630882c75971b7825de28ec"
    end
  end

  on_linux do
    on_arm do
      url "#{RELEASE}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "aa63870085faa75f03a1874c295527b7aee99e70c2aa9bd1665e186b96d48882"
    end
    on_intel do
      url "#{RELEASE}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "b8edd12323acdc2420302558bd026972e5845c88b489e2a26ace4b72ea083f7b"
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
