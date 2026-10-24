#!/bin/bash
# Which serverless hosts run a warp WASM GC module (notes/hosting.md)? Compiles gc_check.warp and wagi_check.warp and
# runs them in local workerd (Cloudflare Workers, wrangler dev), Spin (wasmtime, WAGI) and Deno.
# --edge also deploys the Worker to <account>.workers.dev, calls it, and deletes it again.
# Needs: scripts/own-warp.sh built, wrangler (logged in for --edge), spin ≥ 4, deno, wasm-tools.
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../.." && pwd)
WORK="$ROOT/scratch/hosting_gc"
WARP="$ROOT/scratch/warp"
WORKER_PORT=${WARP_GC_WORKER_PORT:-8897}  # outside the test suite's 87xx ports
SPIN_PORT=${WARP_GC_SPIN_PORT:-8898}
mkdir -p "$WORK/worker" "$WORK/spin"

"$WARP" --no-hints compile --wasm "$HERE/gc_check.warp" >/dev/null && mv "$HERE/gc_check.wasm" "$WORK/"
"$WARP" --no-hints compile --wasm "$HERE/wagi_check.warp" >/dev/null && mv "$HERE/wagi_check.wasm" "$WORK/"

stop_port() { lsof -ti "tcp:$1" | xargs kill 2>/dev/null || true; }

# Cloudflare Workers: main returns a GC struct; /dynamic tries compiling wasm bytes at run time
cat > "$WORK/worker/worker.mjs" <<'EOF'
import program from "../gc_check.wasm";
async function dynamicCompile() {
	try { await WebAssembly.compile(new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0])); return "allowed"; }
	catch (error) { return String(error); }
}
export default {
	async fetch(request) {
		if (new URL(request.url).pathname === "/dynamic") return Response.json({ dynamic: await dynamicCompile() });
		const { exports } = await WebAssembly.instantiate(program, {});
		const result = exports.main();
		return Response.json({ gc: typeof result, kind: String(exports.get_kind(result)) });
	},
};
EOF
printf 'name = "warp-gc-check"\nmain = "worker.mjs"\ncompatibility_date = "2026-09-01"\n' > "$WORK/worker/wrangler.toml"
(cd "$WORK/worker" && wrangler dev --port $WORKER_PORT > "$WORK/wrangler.log" 2>&1 &)
for _ in $(seq 30); do curl -s "localhost:$WORKER_PORT" >/dev/null && break; sleep 1; done
echo "workerd:        $(curl -s localhost:$WORKER_PORT)  $(curl -s localhost:$WORKER_PORT/dynamic)"
stop_port $WORKER_PORT
if [[ "${1:-}" == "--edge" ]]; then
	url=$(cd "$WORK/worker" && wrangler deploy 2>&1 | grep -o 'https://[^ ]*workers.dev')
	sleep 5
	echo "workers.dev:    $(curl -s "$url")  $(curl -s "$url/dynamic")"
	(cd "$WORK/worker" && wrangler delete --force >/dev/null)
fi

# Spin's WAGI wants a void _start; warp's main returns a Node, so a wrapper calls it and drops the result
wasm-tools print "$WORK/wagi_check.wasm" | sed '$ s/)$/  (func (export "_start") call $main drop))/' | wasm-tools parse -o "$WORK/wagi_start.wasm" -
printf 'spin_manifest_version = 2\n[application]\nname = "warp-gc-check"\n[[trigger.http]]\nroute = "/..."\ncomponent = "warp"\nexecutor = { type = "wagi" }\n[component.warp]\nsource = "../wagi_start.wasm"\n' > "$WORK/spin/spin.toml"
(cd "$WORK/spin" && spin up --listen "127.0.0.1:$SPIN_PORT" > "$WORK/spin.log" 2>&1 &)
for _ in $(seq 30); do curl -s "localhost:$SPIN_PORT" >/dev/null && break; sleep 1; done
echo "spin $(spin --version | cut -d' ' -f2):     $(curl -s localhost:$SPIN_PORT)"
stop_port $SPIN_PORT

# Deno (Deno Deploy, Netlify Edge Functions)
cat > "$WORK/deno_check.mjs" <<'EOF'
const { instance } = await WebAssembly.instantiate(await Deno.readFile(Deno.args[0]), {});
const result = instance.exports.main();
console.log(JSON.stringify({ gc: typeof result, kind: String(instance.exports.get_kind(result)) }));
EOF
echo "deno:           $(deno run --allow-read "$WORK/deno_check.mjs" "$WORK/gc_check.wasm")"
