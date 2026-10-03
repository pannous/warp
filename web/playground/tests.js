// The browser test page: lists the tests of the binary (?wasm=, default tests.wasm; ?args= a JSON list of libtest
// filters, `--include-ignored` / `--ignored` included), runs them on a few workers (test-worker.js) and publishes
// window.testSummary for the cargo runner (test_in_browser.py). A test that runs too long is stopped and its worker replaced.

const TEST_TIMEOUT_MS = 120000;
const DEFAULT_WORKERS = 2;

const parameters = new URLSearchParams(location.search);
const wasmUrl = parameters.get("wasm") ?? "tests.wasm";
const libtestArgs = JSON.parse(parameters.get("args") ?? "[]");
const includeIgnored = libtestArgs.includes("--include-ignored");
const onlyIgnored = libtestArgs.includes("--ignored");
const filters = libtestArgs.filter(arg => !["--include-ignored", "--ignored", "--nocapture", "--test-threads"].includes(arg) && !arg.startsWith("--test-threads="));
const workerCount = Number(parameters.get("workers") ?? DEFAULT_WORKERS);

const $ = id => document.getElementById(id);
const results = [];
const running = new Set(); // names of the tests on the workers now: a stuck page names them

function listTests() {
	return new Promise((resolve, reject) => {
		const lister = new Worker("test-worker.js");
		lister.onmessage = ({ data }) => { lister.terminate(); resolve(data.tests); };
		lister.onerror = event => reject(new Error(event.message));
		lister.postMessage({ type: "compile", url: wasmUrl });
		lister.postMessage({ type: "list", filters });
	});
}

const passedResults = () => results.filter(result => result.passed);
const failedResults = () => results.filter(result => !result.passed && !result.skipped);
const skippedResults = () => results.filter(result => result.skipped);

function showProgress(total) {
	const now = running.size ? ` · running ${[...running].join(", ")}` : "";
	$("progress").textContent = `${results.length}/${total} run: ${passedResults().length} passed, ${failedResults().length} failed${now}`;
}

function showFailure(result) {
	const item = document.createElement("li");
	item.className = "warning";
	const label = Object.assign(document.createElement("span"), { className: "label", textContent: result.timedOut ? "timeout" : "failed" });
	const output = Object.assign(document.createElement("pre"), { className: "printed", textContent: result.output.trim().slice(-4000) });
	item.append(label, result.name, output);
	$("failures").append(item);
}

// one worker running tests from the shared queue until it is empty
function runner(queue, total) {
	return new Promise(resolve => {
		let worker, current, timer;
		const record = result => {
			clearTimeout(timer);
			running.delete(result.name);
			results.push(result);
			if (!result.passed && !result.skipped) showFailure(result);
			showProgress(total);
			next();
		};
		const start = () => {
			worker = new Worker("test-worker.js");
			worker.onmessage = ({ data }) => data.type === "result" && record(data);
			worker.postMessage({ type: "compile", url: wasmUrl });
		};
		const next = () => {
			current = queue.shift();
			if (!current) {
				worker.terminate();
				return resolve();
			}
			timer = setTimeout(() => {
				worker.terminate(); // a test that does not stop blocks its worker: replace it
				start();
				record({ name: current.name, passed: false, timedOut: true, output: `stopped after ${TEST_TIMEOUT_MS / 1000} s` });
			}, TEST_TIMEOUT_MS);
			running.add(current.name);
			worker.postMessage({ type: "run", name: current.name, ignored: current.ignored });
		};
		start();
		next();
	});
}

async function main() {
	const started = performance.now();
	const listed = await listTests();
	const selected = listed.filter(test => onlyIgnored ? test.ignored : includeIgnored || !test.ignored);
	const ignored = listed.length - selected.length;
	const queue = [...selected];
	showProgress(selected.length);
	await Promise.all(Array.from({ length: Math.min(workerCount, selected.length || 1) }, () => runner(queue, selected.length)));
	window.testSummary = {
		passed: passedResults().length,
		failed: failedResults().map(({ name, output, timedOut }) => ({ name, output: output.slice(-4000), timedOut: !!timedOut })),
		ignored: ignored + skippedResults().length, // #[should_panic] tests are skipped when panics abort
		total: selected.length,
		seconds: (performance.now() - started) / 1000,
	};
	$("progress").textContent += `, ${ignored} ignored, ${window.testSummary.seconds.toFixed(1)} s`;
}

main().catch(failure => {
	$("progress").textContent = `could not run the tests: ${failure.message}`;
	window.testSummary = { error: failure.message };
});
