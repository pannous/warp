#!/usr/bin/env python3
"""Safari on a macOS CI runner (card webgpu-safari): what the playground does with a GPU sample there.
`safari_probe.py <playground url> <sample>…` drives Safari through safaridriver (classic WebDriver, plain HTTP): opens
the page, waits until it is cross-origin isolated, says what WebGPU the page and a Worker see, then runs each sample
and samples the page every POLL_MS: status, value, the canvases and the output pane's size. CI only (no browser on the
Mac): .github/workflows/safari-probe.yml."""
import json, subprocess, sys, time, urllib.request

PORT = 4445
DRIVER = f"http://127.0.0.1:{PORT}"
ISOLATION_SECONDS = 60
POLL_MS = 500
POLLS = 30

GPU_REPORT = """const done = arguments[arguments.length - 1];
(async () => {
	const report = { isolated: self.crossOriginIsolated, agent: navigator.userAgent, gpu: !!navigator.gpu, offscreen: typeof OffscreenCanvas };
	try {
		const adapter = await navigator.gpu?.requestAdapter();
		report.adapter = !!adapter;
		if (adapter) {
			report.features = [...adapter.features];
			report.limits = Object.fromEntries(["maxTextureDimension2D", "maxBufferSize", "maxStorageBufferBindingSize"].map(name => [name, adapter.limits[name]]));
			const device = await adapter.requestDevice();
			report.device = !!device;
		}
		const worker = new Worker(URL.createObjectURL(new Blob([`(async () => {
			const answer = { gpu: !!navigator.gpu };
			try {
				const adapter = await navigator.gpu?.requestAdapter();
				answer.adapter = !!adapter;
				const canvas = new OffscreenCanvas(4, 4);
				answer.webgpuContext = !!canvas.getContext("webgpu");
			} catch (failure) { answer.error = String(failure); }
			postMessage(answer);
		})()`])));
		report.worker = await new Promise(answered => { worker.onmessage = ({ data }) => answered(data); worker.onerror = event => answered({ error: event.message }); });
		const transferred = document.createElement("canvas").transferControlToOffscreen();
		report.transferable = !!transferred;
	} catch (failure) { report.error = String(failure); }
	done(JSON.stringify(report));
})();"""

PAGE_STATE = """const rect = id => { const r = document.getElementById(id)?.getBoundingClientRect(); return r && [Math.round(r.left), Math.round(r.top), Math.round(r.width), Math.round(r.height)]; };
return JSON.stringify({
	status: document.getElementById("status").textContent,
	value: document.getElementById("value").textContent.slice(0, 300),
	printed: document.getElementById("printed").textContent.slice(-300),
	diagnostics: [...document.querySelectorAll("#diagnostics li")].map(item => item.textContent.slice(0, 300)),
	canvases: [...document.querySelectorAll("#paintings canvas")].map(canvas => `${canvas.width}x${canvas.height} css ${canvas.style.width} gpu ${!!canvas.gpu}`),
	output: rect("output"), painted: rect("painted"), paintings: rect("paintings"),
	errors: (self.probeErrors ?? []).slice(-10),
});"""

CATCH_ERRORS = """self.probeErrors = [];
for (const level of ["error", "warn"]) { const original = console[level]; console[level] = (...parts) => { self.probeErrors.push(`${level}: ${parts.join(" ")}`); original.apply(console, parts); }; }
addEventListener("error", event => self.probeErrors.push(`uncaught: ${event.message}`));
addEventListener("unhandledrejection", event => self.probeErrors.push(`unhandled: ${event.reason}`));"""


def call(method, path, body=None):
	request = urllib.request.Request(f"{DRIVER}{path}", method=method, data=json.dumps(body).encode() if body is not None else None, headers={"Content-Type": "application/json"})
	with urllib.request.urlopen(request, timeout=120) as answer:
		return json.loads(answer.read() or b"{}").get("value")


def main():
	url, samples = sys.argv[1], sys.argv[2:]
	sys.stdout.reconfigure(line_buffering=True)
	driver = subprocess.Popen(["safaridriver", "-p", str(PORT)])
	time.sleep(3)
	session = call("POST", "/session", {"capabilities": {"alwaysMatch": {"browserName": "safari"}}})["sessionId"]
	page = f"/session/{session}"
	run = lambda script: call("POST", f"{page}/execute/sync", {"script": script, "args": []})
	try:
		call("POST", f"{page}/url", {"url": url})
		deadline = time.time() + ISOLATION_SECONDS
		while not run("return document.readyState === 'complete' && self.crossOriginIsolated && !!self.playground") and time.time() < deadline:
			time.sleep(0.5)
		print("gpu:", call("POST", f"{page}/execute/async", {"script": GPU_REPORT, "args": []}))
		run(CATCH_ERRORS)
		print("before:", run(PAGE_STATE))
		for sample in samples:
			run(f"void playground.chooseExample({json.dumps(sample)})")
			for poll in range(POLLS):
				time.sleep(POLL_MS / 1000)
				print(f"{sample} {poll * POLL_MS} ms:", run(PAGE_STATE))
	finally:
		call("DELETE", page)
		driver.terminate()


if __name__ == "__main__":
	main()
