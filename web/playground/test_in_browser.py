#!/usr/bin/env python3
"""Cargo runner for wasm32-wasip1 (.cargo/config.toml): runs the Rust test binary in headless Chrome.
`cargo browser-test [filter…]` builds tests/main.rs without the native feature for wasm32-wasip1 and calls
`test_in_browser.py <tests.wasm> [libtest arguments]`: this serves the repository root and the binary, opens
web/playground/tests.html with agent-browser and prints the results like libtest (exit code 101 on a failure)."""
import functools, http.server, json, os, subprocess, sys, threading, time, urllib.parse

PORT = int(os.environ.get("WARP_BROWSER_TEST_PORT", "8733"))
WORKERS = os.environ.get("WARP_BROWSER_TEST_WORKERS", "2")
SESSION = "warp-browser-tests"
POLL_SECONDS = 3
BINARY_PATH = "/__tests__.wasm"
REPOSITORY = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
IGNORED_ARGUMENTS = ("--nocapture", "--quiet", "-q", "--color", "--format")


def serve(binary):
	class Handler(http.server.SimpleHTTPRequestHandler):
		def translate_path(self, path):
			return binary if urllib.parse.urlsplit(path).path == BINARY_PATH else super().translate_path(path)

		def log_message(self, *_):
			pass
	class Server(http.server.ThreadingHTTPServer):
		request_queue_size = 512  # every worker fetches the binary at once
	server = Server(("127.0.0.1", PORT), functools.partial(Handler, directory=REPOSITORY))
	threading.Thread(target=server.serve_forever, daemon=True).start()
	return server


def browser(*arguments):
	return subprocess.run(["agent-browser", "--session", SESSION, *arguments], capture_output=True, text=True, timeout=300).stdout.strip()


def main():
	binary, arguments = sys.argv[1], [argument for argument in sys.argv[2:] if not argument.startswith(IGNORED_ARGUMENTS)]
	server = serve(binary)
	query = urllib.parse.urlencode({"wasm": BINARY_PATH, "args": json.dumps(arguments), "workers": WORKERS})
	browser("open", f"http://127.0.0.1:{PORT}/web/playground/tests.html?{query}")
	summary, shown = None, ""
	while summary is None:
		time.sleep(POLL_SECONDS)
		progress = browser("get", "text", "#progress")
		if progress != shown:
			print(progress, file=sys.stderr)
			shown = progress
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
