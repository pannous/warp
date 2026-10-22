# Homebrew formula for the tap pannous/homebrew-tap (brew install pannous/tap/warp): warp and warp-runtime (the stub
# standalone executables are made from, next to warp), the prebuilt binaries of the tag's GitHub release (seconds, no
# Rust build); --HEAD builds main from source. Source of truth; the tap gets a copy.
# At a release: packaging/homebrew/update_formula.sh <version> writes the version and the binaries' sha256, see notes/packaging.md
class Warp < Formula
  desc "Wasm-first programming language and data notation (Rust implementation of Wasp)"
  homepage "https://github.com/pannous/warp"
  version "1.4.0"
  license all_of: ["AGPL-3.0-or-later", "MIT"]

  RELEASE = "https://github.com/pannous/warp/releases/download/v#{version}/warp-v#{version}".freeze

  head do
    url "https://github.com/pannous/warp.git", branch: "main"
    depends_on "rust" => :build
  end

  on_macos do
    on_arm do
      url "#{RELEASE}-aarch64-apple-darwin.tar.gz"
      sha256 "9aedcc6ae676b413fae6b82ded84ec7f3bea392f2a89beaf55465429724732d1"
    end
    on_intel do
      url "#{RELEASE}-x86_64-apple-darwin.tar.gz"
      sha256 "931422282fa9a980a90c87eef0a96109ac3de8e32cdd6d292fe22ebb9ddd6412"
    end
  end

  on_linux do
    on_arm do
      url "#{RELEASE}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "b36b26d9e230999aae01123ef6f8a11e7d667a3c68616aabef8f1c00b8b07cf3"
    end
    on_intel do
      url "#{RELEASE}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "fa36213e6948cd5a0cef328464b1a0b33289f290ed78e56885a12f62a6963fd8"
    end
  end

  def install
    if build.head?
      ENV["CARGO_NET_OFFLINE"] = "false" # the repository's .cargo/config.toml builds offline, from the local cache
      system "cargo", "install", *std_cargo_args
      system "cargo", "install", *std_cargo_args(path: "crates/warp-runtime"), "--bin", "warp-runtime"
      system "cargo", "install", *std_cargo_args(path: "crates/warp-runtime"),
             "--features", "webview", "--bin", "warp-page-runtime"
    else
      bin.install "warp", "warp-runtime"
      # releases before the page stub (card build-links) ship none
      bin.install "warp-page-runtime" if File.exist?("warp-page-runtime")
    end
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/warp --version") unless build.head?
    assert_equal "42", shell_output("#{bin}/warp eval '6*7'").strip.split.last
  end
end
