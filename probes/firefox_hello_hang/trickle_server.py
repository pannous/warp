# A static server whose warp.wasm trickles in (RATE bytes per second), and with --stall stops after STALL_AFTER bytes
# without closing: the playground shows the download's progress beside its status, and a note once it made no
# progress for 30 s (card firefox-hello-hang). Run from the repository root:
# python3 probes/firefox_hello_hang/trickle_server.py [port] [--stall | --stall-once], then probes/firefox_hello_hang/progress.sh
import http.server
import os
import sys
import time

PORT = int(next((argument for argument in sys.argv[1:] if argument.isdigit()), 18603))
SLOW_FILE = "/web/playground/warp.wasm"
RATE = 400_000
STALL_AFTER = 1_000_000
STALL = "--stall" in sys.argv
STALL_ONCE = "--stall-once" in sys.argv  # only the first download stalls: the page's restart of a stalled worker loads it
stalled_downloads = []
STALL_SECONDS = 600
ISOLATION = {"Cross-Origin-Embedder-Policy": "require-corp", "Cross-Origin-Opener-Policy": "same-origin"}


class TrickleHandler(http.server.SimpleHTTPRequestHandler):
	def end_headers(self):
		for name, value in ISOLATION.items():
			self.send_header(name, value)
		super().end_headers()

	def do_GET(self):
		if self.path.split("?")[0] != SLOW_FILE:
			return super().do_GET()
		body = open(os.path.join(os.getcwd(), SLOW_FILE.lstrip("/")), "rb").read()
		self.send_response(200)
		self.send_header("Content-Type", "application/wasm")
		self.send_header("Content-Length", str(len(body)))
		self.end_headers()
		stalls = STALL or STALL_ONCE and not stalled_downloads
		stalled_downloads.append(self.path)
		try:
			for offset in range(0, len(body), RATE // 10):
				if stalls and offset >= STALL_AFTER:
					time.sleep(STALL_SECONDS)
				self.wfile.write(body[offset:offset + RATE // 10])
				self.wfile.flush()
				time.sleep(0.1)
		except (BrokenPipeError, ConnectionResetError):
			pass

	def log_message(self, *_):
		pass


class Server(http.server.ThreadingHTTPServer):
	request_queue_size = 512
	daemon_threads = True


Server(("127.0.0.1", PORT), TrickleHandler).serve_forever()
