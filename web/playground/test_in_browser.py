#!/usr/bin/env python3
"""Cargo runner for wasm32-wasip1 (.cargo/config.toml): runs the Rust test binary in headless Chrome.
`cargo browser-test [filter…]` builds tests/main.rs without the native feature for wasm32-wasip1 and calls
`test_in_browser.py <tests.wasm> [libtest arguments]`: this serves the repository root and the binary, opens
web/playground/tests.html with agent-browser and prints the results like libtest (exit code 101 on a failure).
`test_in_browser.py --examples [name…]` runs the tour (examples.js) in the playground page itself, built by build.sh:
each example's value and printed output must be the ones it names (a broken example fails CI, .github/workflows/pages.yml).
`test_in_browser.py --examples --url https://warp.pannous.com/ [name…]` checks the tour of a deployed playground instead.
`test_in_browser.py --examples --firefox [--url …|--site …] [name…]` runs the same check in headless Firefox
(firefox_driver.mjs, WebDriver BiDi) instead of agent-browser's Chrome.
`test_in_browser.py --examples --site _site [name…]` checks it in a collected site (pages.yml), served as its root: a file
the deploy forgot fails here, not on the live page.
`test_in_browser.py --serve [tests.wasm]` only serves (http://127.0.0.1:PORT/web/playground/ and tests.html), for any browser.
Besides the repository it serves /__stub__?status=…&body=… (that response, for fetch tests) and /__include__/<header>: the C header of that name from the first include directory of
src/ffi_parser.rs INCLUDE_DIRS that holds it (the page sets WARP_INCLUDE=/include), nothing else of the machine."""
import functools, http.server, json, os, re, subprocess, sys, threading, time, urllib.parse

# a free port per run (0: the system picks one), so runs of several sessions never meet; WARP_BROWSER_TEST_PORT fixes it
PORT = int(os.environ.get("WARP_BROWSER_TEST_PORT", "0"))
WORKERS = os.environ.get("WARP_BROWSER_TEST_WORKERS", "2")
# the tour's page starts its worker this late (playground.js ?slow_start): a fast machine then meets the races a slow
# CI runner meets, e.g. "ready" hiding the first example's "running…" (empty value, the failed deploys of 2026-10-08)
SLOW_START_MS = 2000
PER_WORKER = os.environ.get("WARP_BROWSER_TEST_PER_WORKER")  # tests before a worker is replaced (tests.js TESTS_PER_WORKER)
# one browser per run: runs at the same time (the suite, a tour check) must never drive or close each other's page
SESSION = f"warp-browser-tests-{os.getpid()}"
LAUNCH_ATTEMPTS = 3
LAUNCH_SECONDS = 90  # a page that is not loaded by then did not start: relaunch, loudly
POLL_SECONDS = 3
BINARY_PATH = "/__tests__.wasm"
INCLUDE_PREFIX = "/__include__/"
HEADER_NAME = re.compile(r"^[\w.+-]+(/[\w.+-]+)*\.h$")
RESULTS_PATH = "/__results__"
LISTING_QUERY = "listing"
STUB_PATH = "/__stub__"  # answers with the status and body its query names, after `delay` milliseconds: the fetch tests' HTTP stub (tests/common serve)
CLICK_MILLISECONDS = 300  # a click's handler runs in the worker and its markup comes back
ISOLATION_SECONDS = 30  # how long a deployed page may take to reload under its service worker
STALL_SECONDS = 300  # no test finished for this long: the page is stuck (a crashed renderer), stop with what is known
REPOSITORY = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
IGNORED_ARGUMENTS = ("--nocapture", "--quiet", "-q", "--color", "--format")
# the console of the page and its workers (console_watch.mjs): any error or warning there fails the tour check
CONSOLE_WATCHER = os.path.join(REPOSITORY, "web", "playground", "console_watch.mjs")
FIREFOX_DRIVER = os.path.join(REPOSITORY, "web", "playground", "firefox_driver.mjs")
firefox = None  # the FirefoxDriver of a --firefox run: browser() and the console go to it instead of agent-browser
CONSOLE_READY = "ready"
CONSOLE_SETTLE_SECONDS = 0.3  # a shown example's last messages reach the watcher
# warnings the browser logs itself, which no page code can silence, and what the program does instead (card
# browser-tour): Chrome without WebGPU says this on every navigator.gpu.requestAdapter(); @gpu then maps on the CPU and
# says so in the program's own warning, gpu_render fails with the program's error (shown on the page, not the console)
BROWSER_OWN_WARNINGS = {"No available adapters."}
PAGE_LOAD = "(loading the page)"


def include_dirs():
	"""the compiler's own list, so both look in the same places"""
	source = open(os.path.join(REPOSITORY, "src", "ffi_parser.rs"), encoding="utf-8").read()
	return re.findall(r'"([^"]+)"', re.search(r"const INCLUDE_DIRS[^=]*=\s*&\[(.*?)\];", source, re.S).group(1))


def find_header(name):
	if not HEADER_NAME.match(name):
		return None
	return next((path for path in (os.path.join(directory, name) for directory in include_dirs()) if os.path.isfile(path)), None)


def serve(binary, root=REPOSITORY):
	class Handler(http.server.SimpleHTTPRequestHandler):
		def translate_path(self, path):
			path = urllib.parse.unquote(urllib.parse.urlsplit(path).path)
			if path == BINARY_PATH and binary:
				return binary
			if path.startswith(INCLUDE_PREFIX):
				return find_header(path[len(INCLUDE_PREFIX):]) or "/nonexistent"
			return super().translate_path(path)

		def send_head(self):
			"""a directory asked with ?listing is listed even when it holds an index.html (wasi.js walks directories)"""
			split = urllib.parse.urlsplit(self.path)
			path = self.translate_path(self.path)
			if split.query == LISTING_QUERY and os.path.isdir(path):
				return self.list_directory(path)
			return super().send_head()

		def do_GET(self):
			split = urllib.parse.urlsplit(self.path)
			if split.path != STUB_PATH:
				return super().do_GET()
			query = urllib.parse.parse_qs(split.query, keep_blank_values=True)
			body = query.get("body", [""])[0].encode()
			code, _, reason = query.get("status", ["200 OK"])[0].partition(" ")
			time.sleep(int(query.get("delay", ["0"])[0]) / 1000)  # milliseconds before the reply: one still pending
			self.send_response(int(code), reason)
			self.send_header("Content-Length", str(len(body)))
			self.end_headers()
			self.wfile.write(body)

		def do_POST(self):
			"""tests.html posts its summary here: scratch/browser_tests_<browser>.json, e.g. from a Firefox tab"""
			if self.path != RESULTS_PATH:
				return self.send_error(404)
			summary = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
			os.makedirs(os.path.join(REPOSITORY, "scratch"), exist_ok=True)
			with open(os.path.join(REPOSITORY, "scratch", f"browser_tests_{summary.get('browser', 'unknown')}.json"), "w") as file:
				json.dump(summary, file, indent=1)
			self.send_response(204)
			self.end_headers()

		def end_headers(self):
			# cross-origin isolation: SharedArrayBuffer for tasks on Workers (host.js startTask)
			self.send_header("Cross-Origin-Opener-Policy", "same-origin")
			self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
			super().end_headers()

		def log_message(self, *_):
			pass
	class Server(http.server.ThreadingHTTPServer):
		request_queue_size = 512  # every worker fetches the binary at once
	global PORT
	try:
		server = Server(("127.0.0.1", PORT), functools.partial(Handler, directory=root))
	except OSError as busy:
		sys.exit(f"error: port {PORT} (WARP_BROWSER_TEST_PORT) is taken{port_holder(PORT)}: {busy.strerror}; unset it for a free port")
	PORT = server.server_address[1]
	threading.Thread(target=server.serve_forever, daemon=True).start()
	return server


def port_holder(port):
	"""` by PID 123 (python3 …)`: the process listening on the port, as lsof names it"""
	found = subprocess.run(["lsof", "-nP", f"-iTCP:{port}", "-sTCP:LISTEN", "-Fpc"], capture_output=True, text=True).stdout.split()
	fields = {line[0]: line[1:] for line in found if line[:1] in ("p", "c")}
	return f" by PID {fields['p']} ({fields.get('c', '?')})" if "p" in fields else ""


def open_page(url):
	"""open `url` in this run's browser and wait until it loaded; a launch that fails is reported and retried, and after
	LAUNCH_ATTEMPTS the run fails with the reason instead of waiting for the stall check"""
	if firefox:
		return firefox.command("open", url) or sys.exit(f"error: Firefox could not open {url}")
	for attempt in range(1, LAUNCH_ATTEMPTS + 1):
		try:
			opened = subprocess.run(["agent-browser", "--session", SESSION, "open", url], capture_output=True, text=True, timeout=LAUNCH_SECONDS)
			reason = opened.stderr.strip() or opened.stdout.strip()
			deadline = time.time() + LAUNCH_SECONDS
			while opened.returncode == 0 and time.time() < deadline:
				if browser("eval", "document.readyState") == '"complete"':
					return
				time.sleep(1)
			reason = reason if opened.returncode else f"the page did not load in {LAUNCH_SECONDS} s"
		except subprocess.TimeoutExpired:
			reason = f"agent-browser open did not return in {LAUNCH_SECONDS} s"
		print(f"error: the browser did not start (attempt {attempt} of {LAUNCH_ATTEMPTS}): {reason}", file=sys.stderr)
		subprocess.run(["agent-browser", "--session", SESSION, "close"], capture_output=True, timeout=60)
	sys.exit(f"error: the browser did not start after {LAUNCH_ATTEMPTS} attempts (agent-browser, session {SESSION})")


def browser(*arguments):
	if firefox:
		return firefox.command(*arguments)
	try:
		return subprocess.run(["agent-browser", "--session", SESSION, *arguments], capture_output=True, text=True, timeout=60).stdout.strip()
	except subprocess.TimeoutExpired:
		return ""  # a busy or crashed page: the stall check decides


# shows an example or sample (%s: its name as JSON) and waits until its run finished
RUN_EXAMPLE = """await playground.chooseExample(%s);
		while (document.getElementById("status").textContent === "running…") await new Promise(done => setTimeout(done, 50));"""


def run_sample(name):
	browser("eval", f"(async () => {{ {RUN_EXAMPLE % json.dumps(name)} }})()")


def reported(message):
	"""whether a console message fails the check: any error or warning but the browser's own BROWSER_OWN_WARNINGS"""
	return not (message["level"] == "warning" and message["text"] in BROWSER_OWN_WARNINGS)


def console_line(message):
	return f"{message['level']}: {message['text']}" + (f" ({message['url']})" if message["url"] else "")


class FirefoxDriver:
	"""headless Firefox (firefox_driver.mjs): a command is a JSON line, its answer one JSON line"""

	def __init__(self):
		self.process = subprocess.Popen(["node", FIREFOX_DRIVER], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
		if self.process.stdout.readline().strip() != '"ready"':
			sys.exit("error: firefox_driver.mjs did not start Firefox")

	def command(self, *arguments):
		self.process.stdin.write(json.dumps(arguments) + "\n")
		self.process.stdin.flush()
		answer = self.process.stdout.readline()
		# close quits the driver without an answer
		if not answer and arguments[0] != "close":
			sys.exit(f"error: firefox_driver.mjs ended (exit code {self.process.wait()}) without answering {arguments[0]}")
		answer = json.loads(answer or '""')
		if isinstance(answer, dict) and "timeout" in answer:
			sys.exit(f"error: Firefox gave {answer['timeout']}")
		return answer


class FirefoxConsole:
	"""the errors and warnings Firefox reported (firefox_driver.mjs), taken per shown example like ConsoleWatch's"""

	def take(self):
		time.sleep(CONSOLE_SETTLE_SECONDS)
		return list(dict.fromkeys(console_line(message) for message in firefox.command("messages") if reported(message)))

	def stop(self):
		pass


class ConsoleWatch:
	"""the errors and warnings of the page's console and its workers' (console_watch.mjs), taken per shown example"""

	def __init__(self):
		self.process = subprocess.Popen(["node", CONSOLE_WATCHER, browser("get", "cdp-url")], stdout=subprocess.PIPE, text=True)
		self.messages, self.ready = [], threading.Event()
		threading.Thread(target=self.read, daemon=True).start()
		if not self.ready.wait(30):
			sys.exit("error: console_watch.mjs did not attach to the browser")

	def read(self):
		for line in self.process.stdout:
			if line.strip() == CONSOLE_READY:
				self.ready.set()
			elif reported(message := json.loads(line)):
				self.messages.append(console_line(message))

	def take(self):
		"""the messages since the last take, each once"""
		time.sleep(CONSOLE_SETTLE_SECONDS)
		taken, self.messages = self.messages, []
		return list(dict.fromkeys(taken))

	def stop(self):
		self.process.terminate()


def show_example(name):
	"""the playground's value and printed text once it showed the example, and its timers ran `wait` milliseconds; with
	`typed` (into the first input) and `clicks` also the value after typing and clicking those buttons or links, whether every element shown stayed (`kept`), kept its key (`keyed`) and whether anything animated (`animated`) and the address bar's path (`address`)"""
	script = f"""(async () => {{
		const name = {json.dumps(name)};
		{RUN_EXAMPLE % json.dumps(name)}
		await new Promise(done => setTimeout(done, EXAMPLES[name].wait ?? 0));
		const shown = {{ value: document.getElementById("value").textContent, printed: document.getElementById("printed").textContent,
			canvases: document.querySelectorAll("#paintings canvas").length }};
		const clicks = EXAMPLES[name].clicks ?? [];
		const shownAddress = () => document.getElementById("address").hidden ? "" : document.getElementById("address").value;
		const typed = EXAMPLES[name].typed;
		if (!clicks.length && typed === undefined) return JSON.stringify(shown);
		const rendered = document.getElementById("rendered").shadowRoot;
		const elements = [...rendered.querySelectorAll("*")];
		const keys = elements.map(element => element.getAttribute("data-warp-key"));
		const field = rendered.querySelector("input");
		let animations = 0;
		const animate = Element.prototype.animate;
		Element.prototype.animate = function (...options) {{ animations += 1; return animate.apply(this, options); }};
		if (typed !== undefined && field) {{
			field.value = typed;
			field.dispatchEvent(new Event("input", {{ bubbles: true, composed: true }}));
			await new Promise(done => setTimeout(done, {CLICK_MILLISECONDS}));
		}}
		for (const text of clicks) {{
			[...rendered.querySelectorAll("button, a")].find(element => element.textContent === text)?.click();
			await new Promise(done => setTimeout(done, {CLICK_MILLISECONDS}));
		}}
		Element.prototype.animate = animate;
		const kept = elements.every(element => element.isConnected);
		const keyed = elements.every((element, index) => keys[index] === null || element.getAttribute("data-warp-key") === keys[index]);
		const clickedPrinted = document.getElementById("printed").textContent;
		return JSON.stringify({{ ...shown, clicked: document.getElementById("value").textContent, clickedPrinted, kept, keyed, animated: animations > 0, address: shownAddress() }});
	}})()"""
	shown = browser("eval", script)
	return json.loads(json.loads(shown)) if shown.startswith('"') else {"value": f"(page gave no answer: {shown})", "printed": ""}


def wait_for_isolation():
	"""a deployed page reloads once under coi-serviceworker.js (index.html) before it is cross-origin isolated: an example
	chosen before that reload is lost; the local server sends the headers itself"""
	deadline = time.time() + ISOLATION_SECONDS
	while browser("eval", "document.readyState === 'complete' && self.crossOriginIsolated") != "true" and time.time() < deadline:
		time.sleep(0.5)


def check_examples(names, page_url=None, site=None):
	"""every example of the tour shows its value and prints its text in the playground (the local build, the collected
	`site` directory, or the deployed one at `page_url`), and neither loading the page nor showing an example or a sample
	writes an error or warning to the console (card console-errors); exit code 101 on a difference"""
	server = None
	if site:
		server = serve(None, os.path.abspath(site))
		page_url = f"http://127.0.0.1:{PORT}/"
	elif not page_url:
		if not os.path.isfile(os.path.join(REPOSITORY, "web", "playground", "warp.wasm")):
			sys.exit("error: web/playground/warp.wasm is missing; build it with web/playground/build.sh")
		server = serve(None)
		page_url = f"http://127.0.0.1:{PORT}/web/playground/"
	open_page("about:blank")
	console = FirefoxConsole() if firefox else ConsoleWatch()  # before the page, so its loading is watched too
	open_page(f"{page_url}{'&' if '?' in page_url else '?'}slow_start={SLOW_START_MS}")
	wait_for_isolation()
	examples = json.loads(json.loads(browser("eval", "JSON.stringify(EXAMPLES)")))
	samples = json.loads(json.loads(browser("eval", "JSON.stringify(Object.keys(SAMPLES).sort())")))
	failures = []
	def verdict(name, wrong):
		wrong += [f"console {message}" for message in console.take()]
		print(f"{'FAIL' if wrong else 'ok  '} {name}" + "".join(f"\n     {line}" for line in wrong))
		if wrong:
			failures.append(name)
	verdict(PAGE_LOAD, [])
	chosen = [(name, False) for name in examples] + [(name, True) for name in samples]
	chosen = [(name, sample) for name, sample in chosen if not names or name in names]
	for name, sample in chosen:
		if sample:
			run_sample(name)
			verdict(f"samples/{name}", [])
		else:
			expected, shown = examples[name], show_example(name)
			verdict(name, [f"{part}: {shown[part]!r}, expected {expected[part]!r}" for part in ("value", "printed", "canvases", "clicked", "clickedPrinted", "kept", "keyed", "animated", "address") if part in expected and shown[part] != expected[part]])
	console.stop()
	browser("close")
	if server:
		server.shutdown()
	print(f"\nexamples and samples: {len(chosen) + 1 - len(failures)} of {len(chosen) + 1} show what they promise, without console errors" + (f"; failed: {', '.join(failures)}" if failures else ""))
	sys.exit(101 if failures else 0)


def build_components():
	"""the components `use wasm` tests call, transpiled for the page (components.js); a failure only warns: those tests
	then fail naming build.sh"""
	built = subprocess.run([os.path.join(REPOSITORY, "web", "playground", "build.sh"), "components"], capture_output=True, text=True)
	if built.returncode:
		print(f"warning: build.sh components failed, `use wasm` tests will fail:\n{built.stderr.strip()}", file=sys.stderr)


def main():
	# each verdict shows in a CI log as it comes, so a stuck run shows where it stuck (card deploy-firefox-hang)
	sys.stdout.reconfigure(line_buffering=True)
	if sys.argv[1:2] == ["--examples"]:
		global firefox
		names = [name for name in sys.argv[2:] if name != "--firefox"]
		firefox = FirefoxDriver() if "--firefox" in sys.argv[2:] else None
		place = {names[0]: names[1]} if names[:1] in (["--url"], ["--site"]) else {}
		check_examples(names[2:] if place else names, place.get("--url"), place.get("--site"))
	if sys.argv[1:2] == ["--serve"]:
		binary = os.path.abspath(sys.argv[2]) if len(sys.argv) > 2 else None
		serve(binary)
		tests = f"tests.html?wasm={BINARY_PATH}" if binary else "tests.html (after copying the binary to web/playground/tests.wasm)"
		print(f"serving {REPOSITORY}\n  playground: http://127.0.0.1:{PORT}/web/playground/\n  tests:      http://127.0.0.1:{PORT}/web/playground/{tests}")
		threading.Event().wait()
	binary, arguments = sys.argv[1], [argument for argument in sys.argv[2:] if not argument.startswith(IGNORED_ARGUMENTS)]
	build_components()
	server = serve(binary)
	query = urllib.parse.urlencode({"wasm": BINARY_PATH, "args": json.dumps(arguments), "workers": WORKERS, **({"perWorker": PER_WORKER} if PER_WORKER else {})})
	open_page(f"http://127.0.0.1:{PORT}/web/playground/tests.html?{query}")
	summary, shown, changed = None, "", time.time()
	while summary is None:
		time.sleep(POLL_SECONDS)
		progress = browser("get", "text", "#progress")
		if progress != shown:
			print(progress, file=sys.stderr)
			shown, changed = progress, time.time()
		elif time.time() - changed > STALL_SECONDS:
			print(f"error: no progress for {STALL_SECONDS} s: {shown}", file=sys.stderr)
			subprocess.run(["agent-browser", "--session", SESSION, "close"], capture_output=True, timeout=60)
			sys.exit(101)
		result = browser("eval", "JSON.stringify(window.testSummary ?? null)")
		summary = json.loads(json.loads(result)) if result.startswith('"') else None
	browser("close")
	server.shutdown()
	if "error" in summary:
		print(f"error: {summary['error']}")
		sys.exit(101)
	for failure in summary["failed"]:
		print(f"\n---- {failure['name']} {'timed out' if failure['timedOut'] else 'failed'} ----\n{failure['output'].strip()}")
	if summary["failed"]:
		print("\nfailures:")
		for failure in summary["failed"]:
			print(f"    {failure['name']}")
	if summary.get("memoryResets"):
		print(f"\nWasm memory ran out {len(summary['memoryResets'])} times; all workers were replaced and these tests ran again: {', '.join(summary['memoryResets'])}")
	verdict = "FAILED" if summary["failed"] else "ok"
	print(f"\ntest result: {verdict}. {summary['passed']} passed; {len(summary['failed'])} failed; {summary['ignored']} ignored; "
		f"finished in {summary['seconds']:.2f}s (in the browser)")
	sys.exit(101 if summary["failed"] else 0)


if __name__ == "__main__":
	main()
