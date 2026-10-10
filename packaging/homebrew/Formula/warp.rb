# Homebrew formula for the tap pannous/homebrew-tap (brew install pannous/tap/warp): warp and warp-runtime (the stub
# standalone executables are made from, next to warp), the prebuilt binaries of the tag's GitHub release (seconds, no
# Rust build); --HEAD builds main from source. Source of truth; the tap gets a copy.
# At a release: packaging/homebrew/update_formula.sh <version> writes the version and the binaries' sha256, see notes/packaging.md
class Warp < Formula
  desc "Wasm-first programming language and data notation (Rust implementation of Wasp)"
  homepage "https://github.com/pannous/warp"
  version "1.2.5"

  RELEASE = "https://github.com/pannous/warp/releases/download/v#{version}/warp-v#{version}".freeze

  head do
    url "https://github.com/pannous/warp.git", branch: "main"
    depends_on "rust" => :build
  end

  on_macos do
    on_arm do
      url "#{RELEASE}-aarch64-apple-darwin.tar.gz"
      sha256 "08d9bd910a7dd20da93ff27f40e0997cbe3bab4149d80cf94a19b01f8f6ddf6f"
    end
    on_intel do
      url "#{RELEASE}-x86_64-apple-darwin.tar.gz"
      sha256 "8c56a92c1f73af3a602fcffd8465f0956da210cebb0981d09ee075024498ea46"
    end
  end

  on_linux do
    on_arm do
      url "#{RELEASE}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "e2069c045806967ee4783f38da54c63b3a3d94da7487a93eb0af502eb8db8c52"
    end
    on_intel do
      url "#{RELEASE}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "760c5049d42ab6e46e8d0bcc6ae01b6f22d28bdf188509c9e18ce3482f8ed78c"
    end
  end

  def install
    if build.head?
      ENV["CARGO_NET_OFFLINE"] = "false" # the repository's .cargo/config.toml builds offline, from the local cache
      system "cargo", "install", *std_cargo_args
      system "cargo", "install", *std_cargo_args(path: "crates/warp-runtime")
    else
      bin.install "warp", "warp-runtime"
    end
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/warp --version") unless build.head?
    assert_equal "42", shell_output("#{bin}/warp eval '6*7'").strip.split.last
  end
end
