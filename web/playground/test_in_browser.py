#!/usr/bin/env python3
"""Cargo runner for wasm32-wasip1 (.cargo/config.toml): runs the Rust test binary in headless Chrome.
`cargo browser-test [filter…]` builds tests/main.rs without the native feature for wasm32-wasip1 and calls
`test_in_browser.py <tests.wasm> [libtest arguments]`: this serves the repository root and the binary, opens
web/playground/tests.html with agent-browser and prints the results like libtest (exit code 101 on a failure).
`test_in_browser.py --serve [tests.wasm]` only serves (http://127.0.0.1:PORT/web/playground/ and tests.html), for any browser.
Besides the repository it serves /__stub__?status=…&body=… (that response, for fetch tests) and /__include__/<header>: the C header of that name from the first include directory of
src/ffi_parser.rs INCLUDE_DIRS that holds it (the page sets WARP_INCLUDE=/include), nothing else of the machine."""
import functools, http.server, json, os, re, subprocess, sys, threading, time, urllib.parse

PORT = int(os.environ.get("WARP_BROWSER_TEST_PORT", "8733"))
WORKERS = os.environ.get("WARP_BROWSER_TEST_WORKERS", "2")
PER_WORKER = os.environ.get("WARP_BROWSER_TEST_PER_WORKER")  # tests before a worker is replaced (tests.js TESTS_PER_WORKER)
SESSION = "warp-browser-tests"
POLL_SECONDS = 3
BINARY_PATH = "/__tests__.wasm"
INCLUDE_PREFIX = "/__include__/"
HEADER_NAME = re.compile(r"^[\w.+-]+(/[\w.+-]+)*\.h$")
RESULTS_PATH = "/__results__"
LISTING_QUERY = "listing"
STUB_PATH = "/__stub__"  # answers with the status and body its query names: the fetch tests' HTTP stub (tests/common serve)
STALL_SECONDS = 300  # no test finished for this long: the page is stuck (a crashed renderer), stop with what is known
REPOSITORY = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
IGNORED_ARGUMENTS = ("--nocapture", "--quiet", "-q", "--color", "--format")


def include_dirs():
	"""the compiler's own list, so both look in the same places"""
	source = open(os.path.join(REPOSITORY, "src", "ffi_parser.rs"), encoding="utf-8").read()
	return re.findall(r'"([^"]+)"', re.search(r"const INCLUDE_DIRS[^=]*=\s*&\[(.*?)\];", source, re.S).group(1))


def find_header(name):
	if not HEADER_NAME.match(name):
		return None
	return next((path for path in (os.path.join(directory, name) for directory in include_dirs()) if os.path.isfile(path)), None)


def serve(binary):
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
	server = Server(("127.0.0.1", PORT), functools.partial(Handler, directory=REPOSITORY))
	threading.Thread(target=server.serve_forever, daemon=True).start()
	return server


def browser(*arguments):
	try:
		return subprocess.run(["agent-browser", "--session", SESSION, *arguments], capture_output=True, text=True, timeout=60).stdout.strip()
	except subprocess.TimeoutExpired:
		return ""  # a busy or crashed page: the stall check decides


def build_components():
	"""the components `use wasm` tests call, transpiled for the page (components.js); a failure only warns: those tests
	then fail naming build.sh"""
	built = subprocess.run([os.path.join(REPOSITORY, "web", "playground", "build.sh"), "components"], capture_output=True, text=True)
	if built.returncode:
		print(f"warning: build.sh components failed, `use wasm` tests will fail:\n{built.stderr.strip()}", file=sys.stderr)


def main():
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
	browser("open", f"http://127.0.0.1:{PORT}/web/playground/tests.html?{query}")
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
