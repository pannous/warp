# Homebrew formula for the tap pannous/homebrew-tap (brew install pannous/tap/warp): the native warp of the tag's GitHub
# release, the folder holding it, its compilers, lib/ and libwarp_wasmtime in libexec (seconds, no build). Source of
# truth; the tap gets a copy.
# At a release: packaging/homebrew/update_formula.sh <version> writes the version and the binaries' sha256, see notes/packaging.md
class Warp < Formula
  desc "Wasm-first programming language and data notation, written in itself"
  homepage "https://github.com/pannous/warp"
  version "1.12.0"
  license all_of: ["AGPL-3.0-or-later", "MIT"]

  depends_on "binaryen" # wasm-opt, which `warp build` trims a program's module with

  RELEASE = "https://github.com/pannous/warp/releases/download/v#{version}/warp-v#{version}".freeze

  on_macos do
    on_arm do
      url "#{RELEASE}-aarch64-apple-darwin.tar.gz"
      sha256 "8b7f965a21bfebd765739cc6653b3f2cd1c368872021b9d0ff7d7b3edb027b77"
    end
    on_intel do
      url "#{RELEASE}-x86_64-apple-darwin.tar.gz"
      sha256 "13e89f92a9659d7be8cd3c574a8c3f567e796bd1580ccbd69e1aab1340653839"
    end
  end

  on_linux do
    on_arm do
      url "#{RELEASE}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "9d69e94f32367e52831d5f526283f34ad31247e83be4a1b52fa6b58fd70f0d1d"
    end
    on_intel do
      url "#{RELEASE}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "57a947c9c97364210200583a63f22908ac279ed78a3febb93bca6969cf3b7351"
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
