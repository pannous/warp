#!/bin/sh
# Dev loop for lib/compiler/emitter.warp: the lowered form of a .warp file (warp lower, read back by parse) goes
# through wasm_module with lib/compiler/runtime.warp lowered alike; the module is written next to scratch/<name>.wasm, validated and run.
# Usage: probes/self_hosting/emitter/emit_lowered.sh <file.warp>   (from the checkout; uses scratch/warp)
set -e
name=$(basename "$1" .warp)
export WARP_NO_WINDOW=1
scratch/warp lower "$(cat "$1")" 2>/dev/null > "scratch/$name.lowered"
scratch/warp lower "$(cat lib/compiler/runtime.warp)" 2>/dev/null > scratch/runtime.lowered
printf 'use "../lib/compiler/emitter.warp"\nm = wasm_module(parse(read("scratch/%s.lowered")), parse(read("scratch/runtime.lowered")))\ntry { write_bytes("scratch/%s.wasm", m) } catch e { print(m) }\n' "$name" "$name" > "scratch/$name.emit.warp"
scratch/warp "scratch/$name.emit.warp" 2>&1 | grep -v '^ \|^warning' || true
[ -f "scratch/$name.wasm" ] && wasm-tools validate "scratch/$name.wasm" && scratch/warp "scratch/$name.wasm"
