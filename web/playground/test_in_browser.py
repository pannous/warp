#!/usr/bin/env python3
"""Cargo runner for wasm32-wasip1 (.cargo/config.toml): runs the Rust test binary in headless Chrome.
`cargo browser-test [filter…]` builds tests/main.rs without the native feature for wasm32-wasip1 and calls
`test_in_browser.py <tests.wasm> [libtest arguments]`: this serves the repository root and the binary, opens
web/playground/tests.html with agent-browser and prints the results like libtest (exit code 101 on a failure).
`test_in_browser.py --serve [tests.wasm]` only serves (http://127.0.0.1:PORT/web/playground/ and tests.html), for any browser.
Besides the repository it serves /__system__/<absolute path> for C headers (*.h under an include directory), which the
FFI header parser reads, as natively."""
import functools, http.server, json, os, re, subprocess, sys, threading, time, urllib.parse

PORT = int(os.environ.get("WARP_BROWSER_TEST_PORT", "8733"))
WORKERS = os.environ.get("WARP_BROWSER_TEST_WORKERS", "2")
SESSION = "warp-browser-tests"
POLL_SECONDS = 3
BINARY_PATH = "/__tests__.wasm"
SYSTEM_PREFIX = "/__system__/"
SYSTEM_FILES = re.compile(r"^/.*/include/.*\.h$")  # C headers only: the page must not read anything else of the machine
STALL_SECONDS = 300  # no test finished for this long: the page is stuck (a crashed renderer), stop with what is known
REPOSITORY = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
IGNORED_ARGUMENTS = ("--nocapture", "--quiet", "-q", "--color", "--format")


def serve(binary):
	class Handler(http.server.SimpleHTTPRequestHandler):
		def translate_path(self, path):
			path = urllib.parse.unquote(urllib.parse.urlsplit(path).path)
			if path == BINARY_PATH and binary:
				return binary
			if path.startswith(SYSTEM_PREFIX):
				system = os.path.normpath("/" + path[len(SYSTEM_PREFIX):])
				return system if SYSTEM_FILES.match(system) else "/nonexistent"
			return super().translate_path(path)

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


def main():
	if sys.argv[1:2] == ["--serve"]:
		binary = os.path.abspath(sys.argv[2]) if len(sys.argv) > 2 else None
		serve(binary)
		tests = f"tests.html?wasm={BINARY_PATH}" if binary else "tests.html (after copying the binary to web/playground/tests.wasm)"
		print(f"serving {REPOSITORY}\n  playground: http://127.0.0.1:{PORT}/web/playground/\n  tests:      http://127.0.0.1:{PORT}/web/playground/{tests}")
		threading.Event().wait()
	binary, arguments = sys.argv[1], [argument for argument in sys.argv[2:] if not argument.startswith(IGNORED_ARGUMENTS)]
	server = serve(binary)
	query = urllib.parse.urlencode({"wasm": BINARY_PATH, "args": json.dumps(arguments), "workers": WORKERS})
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
	verdict = "FAILED" if summary["failed"] else "ok"
	print(f"\ntest result: {verdict}. {summary['passed']} passed; {len(summary['failed'])} failed; {summary['ignored']} ignored; "
		f"finished in {summary['seconds']:.2f}s (in the browser)")
	sys.exit(101 if summary["failed"] else 0)


if __name__ == "__main__":
	main()
