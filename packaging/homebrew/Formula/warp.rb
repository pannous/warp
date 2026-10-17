# Homebrew formula for the tap pannous/homebrew-tap (brew install pannous/tap/warp): warp and warp-runtime (the stub
# standalone executables are made from, next to warp), the prebuilt binaries of the tag's GitHub release (seconds, no
# Rust build); --HEAD builds main from source. Source of truth; the tap gets a copy.
# At a release: packaging/homebrew/update_formula.sh <version> writes the version and the binaries' sha256, see notes/packaging.md
class Warp < Formula
  desc "Wasm-first programming language and data notation (Rust implementation of Wasp)"
  homepage "https://github.com/pannous/warp"
  version "1.3.0"

  RELEASE = "https://github.com/pannous/warp/releases/download/v#{version}/warp-v#{version}".freeze

  head do
    url "https://github.com/pannous/warp.git", branch: "main"
    depends_on "rust" => :build
  end

  on_macos do
    on_arm do
      url "#{RELEASE}-aarch64-apple-darwin.tar.gz"
      sha256 "b734d74819fa5e9b98caf65cc77e26517c335e4f67320fcb8872d7b36612d704"
    end
    on_intel do
      url "#{RELEASE}-x86_64-apple-darwin.tar.gz"
      sha256 "f8b4c32d695975eda58811f3345ce4d40e8ed048d2a6ca7072254e3985b6160a"
    end
  end

  on_linux do
    on_arm do
      url "#{RELEASE}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "8cd5f8650863c702fc3ff0af8c1df6b6bd524a04386414e3d09e95fc29130243"
    end
    on_intel do
      url "#{RELEASE}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "db1e9dacfe18a7ce26f5f2c998e7c7fb9e9f97338da4fccdce7f9741d582caf8"
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
