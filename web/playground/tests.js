// The browser test page: lists the tests of the binary (?wasm=, default tests.wasm; ?args= a JSON list of libtest
// filters, `--include-ignored` / `--ignored` included; ?shard=i/n every n-th of them from the i-th, for CI's matrix), runs them on a few workers (test-worker.js) and publishes
// window.testSummary for the cargo runner (test_in_browser.py). A test that runs too long is stopped and its worker replaced.

const TEST_TIMEOUT_MS = 120000;
// a worker is replaced after this many tests: the Wasm memories of its instances (the test binary's, the programs',
// its task Workers') are freed only by a GC that may come too late, and the page ran out of Wasm memory (flaky-browser)
const TESTS_PER_WORKER = 100;
const DEFAULT_WORKERS = 2;
const RESULTS_URL = "/__results__";

const parameters = new URLSearchParams(location.search);
const wasmUrl = parameters.get("wasm") ?? "tests.wasm";
const libtestArgs = JSON.parse(parameters.get("args") ?? "[]");
const includeIgnored = libtestArgs.includes("--include-ignored");
const onlyIgnored = libtestArgs.includes("--ignored");
const filters = libtestArgs.filter(arg => !["--include-ignored", "--ignored", "--nocapture", "--test-threads"].includes(arg) && !arg.startsWith("--test-threads="));
const workerCount = Number(parameters.get("workers") ?? DEFAULT_WORKERS);
const testsPerWorker = Number(parameters.get("perWorker") ?? TESTS_PER_WORKER);
const [shardIndex, shardCount] = (parameters.get("shard") ?? "1/1").split("/").map(Number);
// round robin, so each shard gets a share of every slow module
const inShard = (_, position) => position % shardCount === shardIndex - 1;

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

// Chrome holds ~124 live Wasm memories per page, all its Workers together, and an isolate that runs out collects only
// the instances it dropped itself (probes/wasm_memory_limit.html): another worker's garbage fails an instance here
const OUT_OF_WASM_MEMORY = /Cannot allocate Wasm memory/;
const runners = new Set(); // the busy runners, each replacing its worker on `replace`
const memoryResets = []; // the tests that ran out of Wasm memory and ran again on fresh workers

// one worker running tests from the shared queue until it is empty
function runner(queue, total) {
	return new Promise(resolve => {
		let worker, current, timer, testsOnWorker;
		const record = result => {
			clearTimeout(timer);
			if (OUT_OF_WASM_MEMORY.test(result.output) && !current.retried) {
				// every worker is replaced, which frees its memories at once, and every test in flight runs again, this one too
				current.retried = true;
				memoryResets.push(current.name);
				return runners.forEach(busy => busy.replace());
			}
			running.delete(result.name);
			results.push(result);
			if (!result.passed && !result.skipped) showFailure(result);
			showProgress(total);
			next();
		};
		const start = () => {
			testsOnWorker = 0;
			worker = new Worker("test-worker.js");
			worker.onmessage = ({ data }) => data.type === "result" && record(data);
			worker.postMessage({ type: "compile", url: wasmUrl });
		};
		const send = () => {
			testsOnWorker++;
			clearTimeout(timer);
			timer = setTimeout(() => {
				worker.terminate(); // a test that does not stop blocks its worker: replace it
				start();
				record({ name: current.name, passed: false, timedOut: true, output: `stopped after ${TEST_TIMEOUT_MS / 1000} s` });
			}, TEST_TIMEOUT_MS);
			running.add(current.name);
			showProgress(total); // a page stuck in this test names it (card browser-suite)
			worker.postMessage({ type: "run", name: current.name, ignored: current.ignored });
		};
		const thisRunner = {
			// a fresh worker, given the test in flight again
			replace: () => {
				worker.terminate();
				start();
				if (running.has(current.name)) send();
			},
		};
		const next = () => {
			current = queue.shift();
			if (!current) {
				runners.delete(thisRunner);
				worker.terminate();
				return resolve();
			}
			if (testsOnWorker >= testsPerWorker) {
				worker.terminate();
				start();
			}
			send();
		};
		runners.add(thisRunner);
		start();
		next();
	});
}

async function main() {
	const started = performance.now();
	const listed = (await listTests()).filter(inShard);
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
		memoryResets,
	};
	$("progress").textContent += `, ${ignored} ignored, ${window.testSummary.seconds.toFixed(1)} s`;
	report(window.testSummary);
}

// the browser's name for the results file test_in_browser.py keeps (a plain static server just refuses the POST)
const browserName = () => /Firefox\//.test(navigator.userAgent) ? "firefox" : /Edg\//.test(navigator.userAgent) ? "edge"
	: /Chrome\//.test(navigator.userAgent) ? "chrome" : /Safari\//.test(navigator.userAgent) ? "safari" : "other";

function report(summary) {
	fetch(RESULTS_URL, { method: "POST", body: JSON.stringify({ ...summary, browser: browserName(), userAgent: navigator.userAgent }) }).catch(() => {});
}

main().catch(failure => {
	$("progress").textContent = `could not run the tests: ${failure.message}`;
	window.testSummary = { error: failure.message };
});
