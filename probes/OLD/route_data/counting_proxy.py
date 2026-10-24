#!/usr/bin/env python3
"""Forwards localhost:<port> to a warp server at localhost:<target> and prints each request, so a probe sees which
server calls a page made, a site Worker's too (the browser's network log misses those). Usage: counting_proxy.py port target"""
import sys
import urllib.error
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

PORT, TARGET = int(sys.argv[1]), int(sys.argv[2])
HOP_HEADERS = {"connection", "transfer-encoding", "content-length", "host"}


class Forwarding(BaseHTTPRequestHandler):
	def forward(self):
		body = self.rfile.read(int(self.headers.get("content-length", 0))) if self.command == "POST" else None
		print(f"{self.command} {urllib.request.unquote(self.path)} {(body or b'').decode()}", flush=True)
		request = urllib.request.Request(f"http://localhost:{TARGET}{self.path}", data=body, method=self.command)
		try:
			reply = urllib.request.urlopen(request)
		except urllib.error.HTTPError as failure:
			reply = failure
		content = reply.read()
		self.send_response(reply.status)
		for name, value in reply.headers.items():
			if name.lower() not in HOP_HEADERS:
				self.send_header(name, value)
		self.send_header("content-length", str(len(content)))
		self.end_headers()
		self.wfile.write(content)

	do_GET = do_POST = forward

	def log_message(self, *_):
		pass


ThreadingHTTPServer(("localhost", PORT), Forwarding).serve_forever()
