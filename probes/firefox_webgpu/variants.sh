#!/bin/sh
# Which program makes Firefox log "uncaught exception: undefined": runs each variant in the playground at <url>, then
# prints its value and the console since the last. Runs in CI (card shaders-firefox): variants.sh <url>
HERE=$(cd "$(dirname "$0")" && pwd)
node -e '
const url = process.argv[1];
const kernel = "@group(0) @binding(0) var<storage, read_write> xs: array<f32>; @compute @workgroup_size(1) fn main() { xs[0] = xs[0] + 1.0; }";
const variants = [
	`gpu_compute("${kernel}", [1], 1)`,
	`gpu_compute("${kernel}", [1], 1)`,
	`try gpu_compute("${kernel}", [1], 1) else 0`,
	`3 times { gpu_compute("${kernel}", [1], 1) }`,
	`try gpu_compute("broken", [1], 1) else 0`,
];
const run = code => `(async () => { await playground.runCode(${JSON.stringify(code)}); while (document.getElementById("status").textContent === "running…") await new Promise(done => setTimeout(done, 50)); return document.getElementById("value").textContent; })()`;
const commands = [["open", url], ["eval", "new Promise(done => setTimeout(done, 3000))"], ["open", url], ["eval", "new Promise(done => setTimeout(done, 5000))"], ["messages"]];
for (const code of variants) commands.push(["eval", `${JSON.stringify("variant: " + code)}`], ["eval", run(code)], ["eval", "new Promise(done => setTimeout(done, 1500))"], ["messages"]);
commands.push(["close"]);
for (const command of commands) console.log(JSON.stringify(command));
' "${1:-http://127.0.0.1:8000/}" | node "$HERE/../../web/playground/firefox_driver.mjs"
