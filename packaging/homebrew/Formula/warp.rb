# Homebrew formula for the tap pannous/homebrew-tap (brew install pannous/tap/warp): the native warp of the tag's GitHub
# release, the folder holding it, its compilers, lib/ and libwarp_wasmtime in libexec (seconds, no build). Source of
# truth; the tap gets a copy.
# At a release: packaging/homebrew/update_formula.sh <version> writes the version and the binaries' sha256, see notes/packaging.md
class Warp < Formula
  desc "Wasm-first programming language and data notation, written in itself"
  homepage "https://github.com/pannous/warp"
  version "1.13.0"
  license all_of: ["AGPL-3.0-or-later", "MIT"]

  depends_on "binaryen" # wasm-opt, which `warp build` trims a program's module with

  RELEASE = "https://github.com/pannous/warp/releases/download/v#{version}/warp-v#{version}".freeze

  on_macos do
    on_arm do
      url "#{RELEASE}-aarch64-apple-darwin.tar.gz"
      sha256 "82ed2942e2cefa984a8357e3651501229a494db729210a04ab4692f815498065"
    end
    on_intel do
      url "#{RELEASE}-x86_64-apple-darwin.tar.gz"
      sha256 "6b3ca03a06af47e371a2af0e4cc1e1882738925f02c644cb9c727abf89d9e6f9"
    end
  end

  on_linux do
    on_arm do
      url "#{RELEASE}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "e28fd5410ecf2968c16335208ab7b5fa3868ce18b8fd90bcd96551c9527af34d"
    end
    on_intel do
      url "#{RELEASE}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "01e53324352063591658e0c265d7eda35dad952e42041e5cef0543276241b0cf"
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
