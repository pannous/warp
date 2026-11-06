# Firefox bug report: terminating a Worker inside WebAssembly.promising (card firefox-jspi-bug)

Ready to paste at https://bugzilla.mozilla.org/enter_bug.cgi?product=Core&component=JavaScript%3A%20WebAssembly
(filing needs the user's Bugzilla account; decision firefox-bug). Attach the repro below as `jspi_terminate.html`.

**Summary:** Terminating a Worker busy inside a WebAssembly.promising (JSPI) call logs a bare "uncaught exception: undefined" that no listener sees

**Steps to reproduce**
1. Open the attached jspi_terminate.html in Firefox (JSPI enabled, `javascript.options.wasm_js_promise_integration`).
2. Open the console. The page starts six Workers spinning in a plain wasm loop and terminates each after 200 ms, then
   the same six times with the loop called through `WebAssembly.promising`.

**Actual results:** each terminate of a Worker busy inside the promising call logs `uncaught exception: undefined` in
the page's console (6 of 6); neither the page's `error` nor its `unhandledrejection` listener sees it. The plain wasm
round logs nothing (0 of 6).

**Expected results:** `Worker.terminate()` ends the Worker silently, as it does for the plain wasm loop, for the same
loop in plain JavaScript, and in Chrome and Safari with or without promising.

**Notes:** found by six repeats per variant in our CI's Firefox run of the warp playground (its Stop button terminates
the program's Worker); the playground's browser test expects the line for Stop until this is fixed
(web/playground/test_in_browser.py STOP_BUTTON).

## Attachment: jspi_terminate.html
```html
<!doctype html>
<title>Firefox: terminating a Worker inside WebAssembly.promising</title>
<!-- Minimal repro for a Firefox bug (card firefox-logs): open in Firefox, watch the console. Each terminate of a Worker
busy inside a WebAssembly.promising call logs a bare "uncaught exception: undefined" on this page, which no
error/unhandledrejection listener sees. The same plain wasm loop without promising logs nothing (the first round), and the same loop in plain JavaScript or with a MessagePort entangled to a second Worker logs nothing either; Chrome and Safari log
nothing either way. Found by six repeats per variant in the Firefox tour (CI run 38670033608). -->
<pre id="log"></pre>
<script>
const REPEATS = 6;
const BUSY_MS = 200;
// (module (func (export "spin") (loop br 0)))
const SPIN_WASM = [0, 97, 115, 109, 1, 0, 0, 0, 1, 4, 1, 96, 0, 0, 3, 2, 1, 0, 7, 8, 1, 4, 115, 112, 105, 110, 0, 0, 10, 9, 1, 7, 0, 3, 64, 12, 0, 11, 11];
const WORKER_SOURCE = `
onmessage = async ({ data: promising }) => {
	const { instance } = await WebAssembly.instantiate(new Uint8Array(${JSON.stringify(SPIN_WASM)}));
	const spin = promising ? WebAssembly.promising(instance.exports.spin) : instance.exports.spin;
	postMessage("busy");
	spin();
};`;
const log = text => document.getElementById("log").textContent += text + "\n";
addEventListener("error", event => log(`error event: ${event.message}`));
addEventListener("unhandledrejection", event => log(`unhandledrejection: ${event.reason}`));
const pause = milliseconds => new Promise(done => setTimeout(done, milliseconds));
const url = URL.createObjectURL(new Blob([WORKER_SOURCE], { type: "text/javascript" }));

async function terminateBusy(promising) {
	for (let time = 0; time < REPEATS; time++) {
		const busy = new Worker(url);
		const started = new Promise(done => busy.onmessage = done);
		busy.postMessage(promising);
		await started;
		await pause(BUSY_MS);
		busy.terminate();
		await pause(BUSY_MS);
	}
	log(`terminated ${REPEATS} busy Workers, promising: ${promising} (see the console)`);
}
terminateBusy(false).then(() => terminateBusy(true));
</script>
```
