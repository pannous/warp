# Binaryen tools

The queued tests put `/opt/homebrew/bin` ahead of inherited system paths on Apple Silicon Macs, while keeping
`~/.cargo/bin` first for Rust. This prevents obsolete Binaryen tools in `/usr/local/bin` from shadowing the current
Homebrew installation. In particular, route-module tests require a `wasm-split` with `--multi-split` support.

When a Binaryen command rejects a supported option, compare every installed copy with `which -a <tool>` and
`<path> --version` before changing the generated WebAssembly or the command line.
