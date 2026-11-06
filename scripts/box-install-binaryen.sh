#!/bin/bash
# Pin Binaryen wasm-opt on the Linux box (card install-binaryen). Matches scripts/ci-compilers.sh BINARYEN=133.
# Idempotent: safe to re-run. See /workspace/handover/box-warp.md § Binaryen.
set -euo pipefail
BINARYEN=${BINARYEN:-133}
TOOLS=${TOOLS:-/home/box/tools}
mkdir -p "$TOOLS"
cd "$TOOLS"
if [ ! -x "$TOOLS/binaryen-version_$BINARYEN/bin/wasm-opt" ]; then
	curl -fsSL -o "binaryen-version_$BINARYEN-x86_64-linux.tar.gz" \
		"https://github.com/WebAssembly/binaryen/releases/download/version_$BINARYEN/binaryen-version_$BINARYEN-x86_64-linux.tar.gz"
	tar -xzf "binaryen-version_$BINARYEN-x86_64-linux.tar.gz"
	rm -f "binaryen-version_$BINARYEN-x86_64-linux.tar.gz"
fi
ln -sfn "$TOOLS/binaryen-version_$BINARYEN/bin/wasm-opt" /home/box/bin/wasm-opt
mkdir -p /home/box/.local/bin
ln -sfn "$TOOLS/binaryen-version_$BINARYEN/bin/wasm-opt" /home/box/.local/bin/wasm-opt
/home/box/bin/wasm-opt --version
