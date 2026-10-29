# Homebrew formula for the tap pannous/homebrew-tap (brew install pannous/tap/warp): the native warp of the tag's GitHub
# release, the folder holding it, its compilers, lib/ and libwarp_wasmtime in libexec (seconds, no build). Source of
# truth; the tap gets a copy.
# At a release: packaging/homebrew/update_formula.sh <version> writes the version and the binaries' sha256, see notes/packaging.md
class Warp < Formula
  desc "Wasm-first programming language and data notation (successor of Wasp, written in warp)"
  homepage "https://github.com/pannous/warp"
  version "1.5.2"
  license all_of: ["AGPL-3.0-or-later", "MIT"]

  depends_on "binaryen" # wasm-opt, which `warp build` trims a program's module with

  RELEASE = "https://github.com/pannous/warp/releases/download/v#{version}/warp-v#{version}".freeze

  on_macos do
    on_arm do
      url "#{RELEASE}-aarch64-apple-darwin.tar.gz"
      sha256 "e93c11953d5f404a9a5ae456da394f62a3b67e9a018be15986df6cdf5c51308c"
    end
    on_intel do
      url "#{RELEASE}-x86_64-apple-darwin.tar.gz"
      sha256 "3a60bcd56b6b3d4ef5219a4ac0fdd92d749828ce66d5ed7f963e4b0a2068c293"
    end
  end

  on_linux do
    on_arm do
      url "#{RELEASE}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "3e9ad17766db6fa82635b62c10b20e8da47a60692665ca335a97941280121d8b"
    end
    on_intel do
      url "#{RELEASE}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "595d8b92900773bbebfdefe704e6fd732675f52f08d76c5e054336aec8b45e79"
    end
  end

  def install
    # the tarball is warp's lib/ with the native warp, its compilers and libwarp_wasmtime in it (decision
    # rust-gone-in-1-5): kept whole, bin/warp a link to its warp, which finds its lib/ and library beside its real path
    libexec.install Dir["*"]
    bin.install_symlink libexec/"warp"
  end

  test do
    assert_equal "42", shell_output("#{bin}/warp '6*7'").strip.split.last
  end
end
