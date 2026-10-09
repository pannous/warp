# Homebrew formula for the tap pannous/homebrew-tap (brew install pannous/tap/warp): warp and warp-runtime (the stub
# standalone executables are made from, next to warp), the prebuilt binaries of the tag's GitHub release (seconds, no
# Rust build); --HEAD builds main from source. Source of truth; the tap gets a copy.
# At a release: packaging/homebrew/update_formula.sh <version> writes the version and the binaries' sha256, see notes/packaging.md
class Warp < Formula
  desc "Wasm-first programming language and data notation (Rust implementation of Wasp)"
  homepage "https://github.com/pannous/warp"
  version "1.2.4"

  RELEASE = "https://github.com/pannous/warp/releases/download/v#{version}/warp-v#{version}".freeze

  head do
    url "https://github.com/pannous/warp.git", branch: "main"
    depends_on "rust" => :build
  end

  on_macos do
    on_arm do
      url "#{RELEASE}-aarch64-apple-darwin.tar.gz"
      sha256 "4cf85eec34919817ecb74c7f76559252d567f3086588493422fdaeb4e1293870"
    end
    on_intel do
      url "#{RELEASE}-x86_64-apple-darwin.tar.gz"
      sha256 "0193923d43f1c0ccc949ad021914c55d9c55ba0cd9e1e771bacceecd0777e6bd"
    end
  end

  on_linux do
    on_arm do
      url "#{RELEASE}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "09852e82ddf5b441e92cbeda602bcf2110b8aca0d474833f4e54d91e5979a9fc"
    end
    on_intel do
      url "#{RELEASE}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "ec299b59c4dabedb882799d8d2485bdeb32fe7e13cbca36ba0b39c5b7ca1c588"
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
