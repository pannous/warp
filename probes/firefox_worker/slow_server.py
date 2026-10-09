# A static server like GitHub Pages (no cross-origin isolation headers, so the page reloads once under
# coi-serviceworker.js) whose worker.js answers late, as over a slow network: a page that starts its Worker before that
# reload has the request aborted (Firefox: NS_BINDING_ABORTED, card firefox-worker).
# Run from the repository root: python3 probes/firefox_worker/slow_server.py [port], then
# python3 web/playground/test_in_browser.py --examples --firefox --url http://localhost:<port>/web/playground/
import http.server
import sys
import time

PORT = int(next((argument for argument in sys.argv[1:] if argument.isdigit()), 18602))
SLOW_FILE = "/web/playground/worker.js"
DELAY_SECONDS = 1.5
VERBOSE = "-v" in sys.argv  # every request with its time: does the page ask for worker.js before its reload?


class SlowWorkerHandler(http.server.SimpleHTTPRequestHandler):
	def do_GET(self):
		if VERBOSE:
			print(f"{time.time():.2f} {self.path}", flush=True)
		if self.path.split("?")[0] == SLOW_FILE:
			time.sleep(DELAY_SECONDS)
		try:
			super().do_GET()
		except (BrokenPipeError, ConnectionResetError):
			print(f"aborted: {self.path}", flush=True)  # the browser dropped the request: the page reloaded meanwhile

	def log_message(self, *_):
		pass


class Server(http.server.ThreadingHTTPServer):
	request_queue_size = 512  # the page's Workers ask at once (test_in_browser.py serve), the default 5 drops some


Server(("127.0.0.1", PORT), SlowWorkerHandler).serve_forever()
