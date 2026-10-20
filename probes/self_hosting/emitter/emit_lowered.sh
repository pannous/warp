#!/bin/sh
# Dev loop for lib/compiler/emitter.warp: the lowered form of a .warp file (`warp lower --json`, the node view the twin
# test reads too) goes through wasm_module with lib/compiler/runtime.warp lowered alike; the module is written to
# scratch/<name>.wasm, validated and run.
# Usage: probes/self_hosting/emitter/emit_lowered.sh <file.warp>   (from the checkout; uses scratch/warp)
set -e
name=$(basename "$1" .warp)
export WARP_NO_WINDOW=1
scratch/warp lower --json "$(cat "$1")" 2>/dev/null > "scratch/$name.lowered.json"
scratch/warp lower --json "$(cat lib/compiler/runtime.warp)" 2>/dev/null > scratch/runtime.lowered.json
printf 'use "../lib/compiler/emitter.warp"\nm = wasm_module(node_of(json(read("scratch/%s.lowered.json"))), node_of(json(read("scratch/runtime.lowered.json"))))\ntry { write_bytes("scratch/%s.wasm", m) } catch e { print(m) }\n' "$name" "$name" > "scratch/$name.emit.warp"
rm -f "scratch/$name.wasm"
scratch/warp --no-hints "scratch/$name.emit.warp" 2>&1 | grep -v '^ \|^warning' || true
[ -f "scratch/$name.wasm" ] && wasm-tools validate "scratch/$name.wasm" && scratch/warp "scratch/$name.wasm"
