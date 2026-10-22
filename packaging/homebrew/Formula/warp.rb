# Homebrew formula for the tap pannous/homebrew-tap (brew install pannous/tap/warp): warp and warp-runtime (the stub
# standalone executables are made from, next to warp), the prebuilt binaries of the tag's GitHub release (seconds, no
# Rust build); --HEAD builds main from source. Source of truth; the tap gets a copy.
# At a release: packaging/homebrew/update_formula.sh <version> writes the version and the binaries' sha256, see notes/packaging.md
class Warp < Formula
  desc "Wasm-first programming language and data notation (Rust implementation of Wasp)"
  homepage "https://github.com/pannous/warp"
  version "1.4.1"
  license all_of: ["AGPL-3.0-or-later", "MIT"]

  RELEASE = "https://github.com/pannous/warp/releases/download/v#{version}/warp-v#{version}".freeze

  head do
    url "https://github.com/pannous/warp.git", branch: "main"
    depends_on "rust" => :build
  end

  on_macos do
    on_arm do
      url "#{RELEASE}-aarch64-apple-darwin.tar.gz"
      sha256 "ece900cd9d84beb8dbed4c519a44f83bd679705aa6263b69a6e9a1a0265d9ba0"
    end
    on_intel do
      url "#{RELEASE}-x86_64-apple-darwin.tar.gz"
      sha256 "cba7fecc8b4a41f7b8945e45c1ac6c820482b988bb0716668cf4ad5065bb4de1"
    end
  end

  on_linux do
    on_arm do
      url "#{RELEASE}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "7ee117f887deb93cb35ced09069595c8a27092fbf94bf6eeea74ab9243e48b5b"
    end
    on_intel do
      url "#{RELEASE}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "0e340390ec00cabad9a92f169194560ebb506125db4c40d0d67cf36c4f29da7f"
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
