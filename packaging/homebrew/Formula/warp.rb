# Homebrew formula for the tap pannous/homebrew-tap (brew install pannous/tap/warp): warp and warp-runtime (the stub
# `warp build --exe` needs next to warp), built from the tagged source. Source of truth; the tap gets a copy.
# At a release: url + sha256 of the new tag's archive, see notes/packaging.md
class Warp < Formula
  desc "Wasm-first programming language and data notation (Rust implementation of Wasp)"
  homepage "https://github.com/pannous/warp"
  url "https://github.com/pannous/warp/archive/refs/tags/v0.1.1.tar.gz"
  sha256 "SHA256_OF_TAG_ARCHIVE"
  head "https://github.com/pannous/warp.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
    system "cargo", "install", *std_cargo_args(path: "crates/warp-runtime")
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/warp --version")
    assert_equal "42", shell_output("#{bin}/warp eval '6*7'").strip.split.last
  end
end
