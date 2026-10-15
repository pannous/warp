#!/bin/sh
# Is Firefox's anonymous "uncaught exception: undefined" during many GPU jobs (test_in_browser.py GPU_JOBS) the
# Worker-terminated-inside-WebAssembly.promising bug (probes/firefox_jspi_terminate.html, card firefox-logs)? Logs each
# Worker.terminate of the page with its stack, then runs job counts and prints the console after each, so a terminate
# right before the exception points to it (card firefox-gpu-jobs-uncaught). Runs in CI: gpu_jobs.sh <url>
HERE=$(cd "$(dirname "$0")" && pwd)
node -e '
const url = process.argv[1];
const kernel = "@group(0) @binding(0) var<storage, read_write> xs: array<f32>; @compute @workgroup_size(1) fn main() { xs[0] = xs[0] + 1.0; }";
const jobs = (count, tried) => `shader = "${kernel}"\n${count} times { ${tried ? "try gpu_compute(shader, [1], 1) else 0" : "gpu_compute(shader, [1], 1)"} }\ngpu_compute(shader, [1], 1)`;
const variants = [jobs(1, false), jobs(50, true), jobs(400, false), jobs(400, true), jobs(400, true)];
const logTerminations = `{ const terminate = Worker.prototype.terminate; Worker.prototype.terminate = function () { console.log("terminate " + new Error().stack.split("\\n").slice(1, 4).join(" < ")); return terminate.call(this); }; "logging terminate" }`;
const run = code => `(async () => { await playground.runCode(${JSON.stringify(code)}); while (document.getElementById("status").textContent === "running…") await new Promise(done => setTimeout(done, 50)); return document.getElementById("value").textContent; })()`;
const commands = [["open", url], ["eval", "new Promise(done => setTimeout(done, 3000))"], ["open", url], ["eval", "new Promise(done => setTimeout(done, 5000))"], ["eval", logTerminations], ["messages"]];
for (const code of variants) commands.push(["eval", JSON.stringify("variant: " + code.split("\n")[1])], ["eval", run(code)], ["eval", "new Promise(done => setTimeout(done, 1500))"], ["messages"]);
commands.push(["close"]);
for (const command of commands) console.log(JSON.stringify(command));
' "${1:-http://127.0.0.1:8000/}" | node "$HERE/../../web/playground/firefox_driver.mjs"
