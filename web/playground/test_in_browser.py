#!/usr/bin/env python3
"""Cargo runner for wasm32-wasip1 (.cargo/config.toml): runs the Rust test binary in headless Chrome.
`cargo browser-test [filter…]` builds tests/main.rs without the native feature for wasm32-wasip1 and calls
`test_in_browser.py <tests.wasm> [libtest arguments]`: this serves the repository root and the binary, opens
web/playground/tests.html with agent-browser and prints the results like libtest (exit code 101 on a failure).
`test_in_browser.py --examples [name…]` runs the tour (examples.js) in the playground page itself, built by build.sh:
each example's value and printed output must be the ones it names (a broken example fails CI, .github/workflows/pages.yml).
`test_in_browser.py --examples --url https://warp.pannous.com/ [--site _site] [name…]` checks the tour of a deployed
playground instead, with --site once it serves those built files.
`test_in_browser.py --examples --firefox [--url …|--site …] [name…]` runs the same check in headless Firefox
(firefox_driver.mjs, WebDriver BiDi) instead of agent-browser's Chrome; `--safari` in Safari (safaridriver, classic
WebDriver: macOS CI runners, after `sudo safaridriver --enable`; its console is not watched).
`test_in_browser.py --examples --site _site [name…]` checks it in a collected site (pages.yml), served as its root: a file
the deploy forgot fails here, not on the live page.
`test_in_browser.py --serve [tests.wasm]` only serves (http://127.0.0.1:PORT/web/playground/ and tests.html), for any browser.
Besides the repository it serves /__stub__?status=…&body=… (that response, for fetch tests) and /__include__/<header>: the C header of that name from the first include directory of
src/formats/ffi_parser.rs INCLUDE_DIRS that holds it (the page sets WARP_INCLUDE=/include), nothing else of the machine."""
import functools, http.server, json, os, re, subprocess, sys, threading, time, urllib.parse, urllib.request

# a free port per run (0: the system picks one), so runs of several sessions never meet; WARP_BROWSER_TEST_PORT fixes it
PORT = int(os.environ.get("WARP_BROWSER_TEST_PORT", "0"))
WORKERS = os.environ.get("WARP_BROWSER_TEST_WORKERS", str(os.cpu_count() or 2))  # tests in flight: one per core
SHARD = os.environ.get("WARP_BROWSER_TEST_SHARD")  # i/n: every n-th test from the i-th (pages.yml's matrix)
# the tour's page starts its worker this late (playground.js ?slow_start): a fast machine then meets the races a slow
# CI runner meets, e.g. "ready" hiding the first example's "running…" (empty value, the failed deploys of 2026-10-08)
SLOW_START_MS = 2000
COMPONENTS_BUILT = os.environ.get("WARP_COMPONENTS_BUILT") == "1"
PER_WORKER = os.environ.get("WARP_BROWSER_TEST_PER_WORKER")  # tests before a worker is replaced (tests.js TESTS_PER_WORKER)
# one browser per run: runs at the same time (the suite, a tour check) must never drive or close each other's page
SESSION = f"warp-browser-tests-{os.getpid()}"
LAUNCH_ATTEMPTS = 3
LAUNCH_SECONDS = 90  # a page that is not loaded by then did not start: relaunch, loudly
# `--url … --site _site`: the deployed site is checked once it serves the built files, every one of them (card
# playground-verify: right after a deploy the CDN served a mix of old and new files, 58 examples failing)
DEPLOYED_WAIT_SECONDS = 600
DEPLOYED_POLL_SECONDS = 10
POLL_SECONDS = 3
BINARY_PATH = "/__tests__.wasm"
INCLUDE_PREFIX = "/__include__/"
HEADER_NAME = re.compile(r"^[\w.+-]+(/[\w.+-]+)*\.h$")
RESULTS_PATH = "/__results__"
LISTING_QUERY = "listing"
STUB_PATH = "/__stub__"  # answers with the status and body its query names, after `delay` milliseconds: the fetch tests' HTTP stub (tests/common serve)
CLICK_MILLISECONDS = 300  # a click's handler runs in the worker and its markup comes back
ISOLATION_SECONDS = 30  # how long a deployed page may take to reload under its service worker
# serve without the isolation headers, as GitHub Pages does, so the page isolates itself under coi-serviceworker.js
# (card firefox-coi-worker: probes/firefox_start/starts.py follows those starts)
UNISOLATED = os.environ.get("WARP_SERVE_UNISOLATED") == "1"
STALL_SECONDS = 300  # no test finished for this long: the page is stuck (a crashed renderer), stop with what is known
REPOSITORY = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
CI_ONLY = "browser tests run in CI only: skipped here, Chrome is not started outside CI (dispatch the Playground workflow on your branch)"
IGNORED_ARGUMENTS = ("--nocapture", "--quiet", "-q", "--color", "--format")
# the console of the page and its workers (console_watch.mjs): any error or warning there fails the tour check
CONSOLE_WATCHER = os.path.join(REPOSITORY, "web", "playground", "console_watch.mjs")
FIREFOX_DRIVER = os.path.join(REPOSITORY, "web", "playground", "firefox_driver.mjs")
webdriver = None  # the FirefoxDriver or SafariDriver of a --firefox or --safari run: browser() and the console go to it instead of agent-browser
SAFARI_PORT = 4445
SAFARI_START_SECONDS = 10
SAFARI_SCRIPT_MS = 300_000  # an example that runs this long is stuck
# agent-browser's eval for classic WebDriver: the expression's value, awaited, as JSON text ("" for undefined)
SAFARI_EVAL = """const done = arguments[arguments.length - 1];
Promise.resolve().then(() => (0, eval)(arguments[0])).then(value => done(value === undefined ? "" : JSON.stringify(value)), failure => done(`✗ ${failure?.message ?? failure}`));"""
CONSOLE_READY = "ready"
CONSOLE_SETTLE_SECONDS = 0.3  # a shown example's last messages reach the watcher
# warnings the browser logs itself, which no page code can silence, and what the program does instead (card
# browser-tour): Chrome without WebGPU says this on every navigator.gpu.requestAdapter(); @gpu then maps on the CPU and
# says so in the program's own warning, gpu_render fails with the program's error (shown on the page, not the console)
# Chrome's Dawn says the second when the next example ends the task Worker of an animating GPU sample while a frame of
# its OffscreenCanvas is in flight (samples/manual_3d_object_viewer, cards flaky-viewer, flaky-deploy): the stopped
# program's device is gone, nothing the next example sees
BROWSER_OWN_WARNINGS = {"No available adapters.", "A valid external Instance reference no longer exists."}
# an example on the GPU (paint of a shader) in a browser without WebGPU (the runner's Firefox): skipped, loudly
NO_GPU = "offers no WebGPU adapter"
PAGE_LOAD = "(loading the page)"
TEXT_HOLE = "(a computed text between children changes alone)"  # card web-fine-text-nodes: markup.rs text_holes
TEXT_HOLE_CODE = 'n = 0\ndiv{ section{ h1{ "Counter" } button{ on click { n += 1 } "add" } "clicked " + n + " times" } }'
TEXT_HOLE_CLICKS = 3
TEXT_HOLE_SHOWN = "Counteraddclicked 3 times"
COMPONENT_HOLE = "(a component instance's text changes alone)"  # card web-fine-components: component_state.rs hole_bindings
COMPONENT_HOLE_CODE = 'def Tally(start) {\n\tn = start\n\tsection{ button{ on click { n += 1 } "add" } "clicked " + n + " times" }\n}\ndiv{ Tally(2) Tally(7) }'
COMPONENT_HOLE_SHOWN = "addclicked 2 timesaddclicked 10 times"
REPORT = "(?report shows the report button, which drafts an issue with the program)"  # card report-button
REPORT_CODE = 'print "reported"'
REPORT_PRINTED = "reported"
FILE_KEPT = "(a written file after a reload)"  # card files-written: host-files.js keeps written files in IndexedDB
FILE_KEPT_TEXT = "kept across a reload"
WRITE_FILE = f'use file\nwrite("kept.txt", "{FILE_KEPT_TEXT}")'
READ_FILE = 'use file\nread("kept.txt")'
COMPLETION_KEYS = "(completion with Tab and Enter)"  # cards code-completion-must, completion-must (completion.js)
# code at the cursor, the keys pressed, the code after them
COMPLETIONS = [
	("use math\ndir(ma", ["Tab", "Tab"], "use math\ndir(math"),
	("xs_long = 1\nxs_short = 2\nxs_", ["Tab", "Enter"], "xs_long = 1\nxs_short = 2\nxs_long"),
	("xs = 1\nxs_a = 2\nxs_b = 3\nxs", ["Tab", "Enter"], "xs = 1\nxs_a = 2\nxs_b = 3\nxs\n"),
]
KEY_CODES = {"Tab": 9, "Enter": 13}
# card playground-turn: a long run turns Run into Stop, which stops it
STOP_BUTTON = "(Run becomes Stop while a program runs, and stops it)"
STOP_BUTTON_CODE = "n = 0\nwhile yes { n += 1 }"
STOP_BUTTON_WAIT_MS = 1500  # past playground.js STOP_GRACE_MS, well before RUN_TIMEOUT_MS
MEMBER_COMPLETION = "(members listed after a dot)"  # card member-completion: completion.js asks the compiler (members.rs)
MEMBER_CODE = "xs = [1 2]; xs"
MEMBERS_LISTED = ["count", "sum", "map", "unique"]
MEMBERS_WAIT_MS = 10000
ASSISTANT_PAUSE_MS = 1500  # assistant.js PAUSE_BEFORE_COMPLETION_MS and a margin
PAGE_ELEMENTS = "(a handler reads the page's element of an id: $who.value)"  # card browser-interaction: host-foreign.js callOnPage
PAGE_ELEMENTS_CODE = 'greeting = "-"\ndiv{ input{ id: "who" } button{ on click { greeting = "Hello " + $who.value } "Greet" } p{ greeting } }'
PAGE_ELEMENTS_NAME = "Ada"
PAGE_ELEMENTS_SHOWN = "Hello Ada"
CLIPBOARD_READ = "(a program reads the clipboard)"  # card web-apis-rest: host.js clipboard_text asks the page
CLIPBOARD_TEXT = "copied by the page"
# card web-apis-rest: gpu_compute over named arrays, and paint of a shader drawn straight into the page's canvas
GPU_PAGE = "(named GPU arrays, a shader painted into the page's canvas)"
SHADER_SCALED = "scaled auto"  # card make-shader: not the pixelated squares of pixels painted one by one
NAMED_ARRAYS = """shader = "@group(0) @binding(0) var<storage, read_write> xs: array<f32>;
@group(0) @binding(1) var<storage, read_write> ns: array<i32>;
@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id: vec3<u32>) {
	if (id.x < arrayLength(&xs)) { xs[id.x] = xs[id.x] + 10.0; ns[id.x] = ns[id.x] - 1; }
}"
gpu_compute(shader, {xs: [1.5 2 3], ns: [7 8 9]}, 1)"""
NAMED_ARRAYS_SHOWN = ["11.5", "6 7 8"]
RED_SHADER = 'paint("@fragment fn main(@builtin(position) at: vec4f) -> @location(0) vec4f { return vec4f(1.0, 0.0, 0.0, 1.0); }", 8, 8)'
# card wgsl-error: a WGSL error of a shader block names its line and column in the program, whose word there is marked
SHADER_ERROR = "(a WGSL error at its place in the program)"
SHADER_ERROR_CODE = """broken = shader {
	@fragment fn main(@builtin(position) at: vec4f) -> @location(0) vec4f {
		return vec4f(level(at.x), 0.0, 0.0, 1.0);
	}
	fn level(band: f32) : f32 {
		return band / 8.0;
	}
}
paint(broken, 8, 8)"""
SHADER_ERROR_PLACE = "at 5:22"
SHADER_EXCERPT = "┌─"  # naga's excerpt of the shader (Firefox): the error says why in one line instead (card shader-errors)
CANVAS_FAILED = "the page's canvas failed"  # host-gpu.js CANVAS_FAILED: the shader painted through pixels instead
CANVAS_SETTLE_MS = 300  # the task Worker's frame reaches the page's canvas after its task
# card webgpu-safari: a paint of the view's size makes the output pane taller; the view's size must not grow with it
# (it did by ~40 px a frame, until Safari ran out of memory painting samples/webgpu)
VIEW_STEADY = "(the view's size stays as paintings of that size are shown)"
VIEW_STEADY_CODE = """paint(int[view_width * view_height], view_width, view_height)
[view_width, view_height]"""
VIEW_STEADY_RUNS = 4  # a run measures the view as the last one's painting left the page; the first run's painting may
# bring a scrollbar or the value's line once, after which the size must hold (it grew ~40 px a run before the fix)
# card webgpu-safari: a shared buffer per GPU job ran Safari out of memory after ~200 jobs (a frame each; CI's runners
# have no GPU adapter, so this guards the jobs' path without reproducing that); the jobs fail
# without an adapter too, which `try` takes, and the last one then shows its own failure, never "Out of memory"
GPU_JOBS = "(many GPU jobs in one run)"
GPU_JOBS_CODE = """shader = "@group(0) @binding(0) var<storage, read_write> xs: array<f32>; @compute @workgroup_size(1) fn main() { xs[0] = xs[0] + 1.0; }"
400 times { try gpu_compute(shader, [1], 1) else 0 }
gpu_compute(shader, [1], 1)"""
GPU_JOBS_NEVER = "Out of memory"
# the console lines a check makes on purpose: Chrome logs the broken shader's WGSL error itself
# Firefox on lavapipe: the 400 jobs outlast RUN_TIMEOUT_MS, so the run is stopped and the stop's terminate logs the
# anonymous uncaught exception below; 50 jobs finish in time and log nothing (probes/firefox_webgpu/gpu_jobs.sh, card
# firefox-gpu-jobs-uncaught)
# and Firefox its own two lines of the broken shader
# Firefox's bug: terminating a Worker busy inside a WebAssembly.promising call logs an anonymous uncaught exception on
# the page, which no listener sees (probes/firefox_jspi_terminate.html, card firefox-logs). Stop does that: the
# compiler's web_evaluate and the program's main run under promising (worker.js, host.js JSPI); the run stopped and
# the button right
EXPECTED_CONSOLE = {SHADER_ERROR: ["Error while parsing WGSL", "errors while creating shader module", "parsing error"], GPU_JOBS: ["uncaught exception: undefined"],
	STOP_BUTTON: ["uncaught exception: undefined"]}
# card safari-shared-buffers: a task's answer buffer, like a GPU job's, is reused once read
MANY_TASKS = "(many tasks in one run)"
MANY_TASKS_CODE = """twice(x) := x * 2
total = 0
for i in 1 to 400 { total += await go twice(i) }
total"""
MANY_TASKS_SHOWN = "160400"
# card shader-brush: samples/shader_brush.warp colors the paper under the pointer (its $mouse), not only a corner
BRUSH = "(samples/shader_brush colors the paper under the pointer)"
BRUSH_SAMPLE = "shader_brush"
BRUSH_WAIT_MS = 8000  # the sample paints a frame every 16 ms; a software GPU takes far longer for its 1280×800
BRUSH_POLL_MS = 250
PAPER_FROM = 240  # a channel this light or lighter on all three: the sample's paper (0.98), no brush
# card register-fingers: a finger dragged over samples/finger_paint's canvas paints a stroke (the iPad's Safari took the
# touch over for scrolling). Then, only told, a drag over the canvas set back to the browser's touch handling, on a
# part the first stroke didn't reach: whether this browser takes a touch over at all (else the check can't fail); it
# goes last, as Chrome's swipe went back in the page's history, to another example
FINGERS = "(a finger paints in samples/finger_paint)"
FINGERS_SAMPLE = "finger_paint"
FINGER_DRAG = os.path.join(os.path.dirname(os.path.abspath(__file__)), "touch_drag.mjs")
FINGERS_BROWSER_TOUCH = (0.1, 0.4)
FINGERS_PAINTED = (0.6, 0.9)
FINGERS_WAIT_MS = 5000
# cards https-pannous, very-fast: the `on frame` shader samples keep painting, as the user saw them stop at once in
# Chromium and Safari; their canvas still changes between two looks, the pointer moving over it
FRAME_SHADERS = "(the on frame shader samples keep painting)"
FRAME_SHADER_SAMPLES = ["shader_brush", "shader_trails"]
FRAME_SHADER_FIRST_MS = 15000  # the first frame's canvas: a software GPU takes long for a 1280×800 frame
FRAME_SHADER_LOOK_AGAIN_MS = 3000
THROUGH_PIXELS = "through pixels, not into the page's canvas"  # Chrome's SwiftShader drops the page canvas
# card shader-read: $previous(at) reads the frame the shader left before, as natively (tests/programs/
# test_shader_builtins.rs a_shader_reads_its_previous_frame): three renders of a quarter red more give 192 red
PREVIOUS_FRAME = "(a shader reads its previous frame)"
PREVIOUS_FRAME_CODE = """use graphics
s = shader {
	@fragment fn main(@builtin(position) at: vec4f) -> @location(0) vec4f {
		return $previous(at.xy) + vec4f(0.25, 0.0, 0.0, 1.0);
	}
}
gpu_render(s, 2, 2)
gpu_render(s, 2, 2)
gpu_render(s, 2, 2)#2"""
PREVIOUS_FRAME_SHOWN = str(0xFFC0_0000)
# each GET the server began and finished, (seconds since it started, path, finished): shown after a failed tour, which
# then says whether the browser asked for a stalled worker's scripts at all (card tour-firefox)
served = []
SERVED_SHOWN = 400  # the first requests: the page's and its workers' loading


@functools.cache
def include_dirs():
	"""the compiler's own list, so both look in the same places; read when the server starts, so a moved ffi_parser.rs
	stops the run instead of failing each header request quietly"""
	source = open(os.path.join(REPOSITORY, "src", "formats", "ffi_parser.rs"), encoding="utf-8").read()
	return re.findall(r'"([^"]+)"', re.search(r"const INCLUDE_DIRS[^=]*=\s*&\[(.*?)\];", source, re.S).group(1))


def find_header(name):
	if not HEADER_NAME.match(name):
		return None
	return next((path for path in (os.path.join(directory, name) for directory in include_dirs()) if os.path.isfile(path)), None)


def log_served(path, finished):
	served.append((time.time() - SERVER_STARTED, path, finished))


def show_served():
	print(f"the server's first {SERVED_SHOWN} requests (s since it started, → asked, ✓ answered):")
	for seconds, path, finished in served[:SERVED_SHOWN]:
		print(f"  {seconds:7.2f} {'✓' if finished else '→'} {path}")


SERVER_STARTED = time.time()


def serve(binary, root=REPOSITORY):
	include_dirs()

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
				log_served(self.path, False)
				super().do_GET()
				return log_served(self.path, True)
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
			if urllib.parse.urlsplit(self.path).path == STUB_PATH:
				return self.echo()
			if self.path != RESULTS_PATH:
				return self.send_error(404)
			summary = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
			os.makedirs(os.path.join(REPOSITORY, "scratch"), exist_ok=True)
			with open(os.path.join(REPOSITORY, "scratch", f"browser_tests_{summary.get('browser', 'unknown')}.json"), "w") as file:
				json.dump(summary, file, indent=1)
			self.send_response(204)
			self.end_headers()

		def echo(self):
			"""a POST to the stub answers what it got, as httpbin.org/post does: {data, headers} (tests/common echo_url)"""
			data = self.rfile.read(int(self.headers.get("Content-Length", 0))).decode()
			headers = {"-".join(part.capitalize() for part in name.split("-")): value for name, value in self.headers.items()}
			body = json.dumps({"data": data, "headers": headers}).encode()
			self.send_response(200)
			self.send_header("Content-Type", "application/json")
			self.send_header("Content-Length", str(len(body)))
			self.end_headers()
			self.wfile.write(body)

		def end_headers(self):
			# cross-origin isolation: SharedArrayBuffer for tasks on Workers (host.js startTask)
			if not UNISOLATED:
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
	if webdriver:
		return webdriver.command("open", url) or sys.exit(f"error: {webdriver.NAME} could not open {url}")
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


browser_complaint = ""  # what agent-browser said on stderr at its last command


def browser(*arguments):
	if webdriver:
		return webdriver.command(*arguments)
	global browser_complaint
	try:
		ran = subprocess.run(["agent-browser", "--session", SESSION, *arguments], capture_output=True, text=True, timeout=60)
	except subprocess.TimeoutExpired:
		browser_complaint = f"agent-browser {arguments[0]} took over 60 s"
		return ""  # a busy or crashed page: the stall check decides
	browser_complaint = ran.stderr.strip()
	return ran.stdout.strip()


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
	NAME = "Firefox"

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


class SafariDriver:
	"""Safari through safaridriver (classic WebDriver over HTTP): open, eval and close as FirefoxDriver's; no console"""
	NAME = "Safari"

	def __init__(self):
		self.driver = subprocess.Popen(["safaridriver", "-p", str(SAFARI_PORT)])
		deadline = time.time() + SAFARI_START_SECONDS
		while True:
			try:
				self.session = self.call("POST", "/session", {"capabilities": {"alwaysMatch": {"browserName": "safari"}}})["sessionId"]
				break
			except OSError as failure:
				if time.time() > deadline:
					sys.exit(f"error: safaridriver did not start Safari: {failure}")
				time.sleep(0.5)
		self.call("POST", f"/session/{self.session}/timeouts", {"script": SAFARI_SCRIPT_MS})

	def call(self, method, path, body=None):
		request = urllib.request.Request(f"http://127.0.0.1:{SAFARI_PORT}{path}", method=method, data=None if body is None else json.dumps(body).encode(), headers={"Content-Type": "application/json"})
		with urllib.request.urlopen(request, timeout=SAFARI_SCRIPT_MS / 1000 + 60) as answer:
			return json.loads(answer.read() or b"{}").get("value")

	def command(self, name, *arguments):
		page = f"/session/{self.session}"
		if name == "open":
			self.call("POST", f"{page}/url", {"url": arguments[0]})
			return True
		if name == "eval":
			try:
				return self.call("POST", f"{page}/execute/async", {"script": SAFARI_EVAL, "args": [arguments[0]]})
			except urllib.error.HTTPError as failure:
				return f"✗ {failure.read().decode(errors='replace')[:500]}"
		if name == "messages":
			return []
		if name == "close":
			self.call("DELETE", page)
			self.driver.terminate()
			return ""
		sys.exit(f"error: test_in_browser.py drives Safari without {name}")


class WebDriverConsole:
	"""the errors and warnings Firefox reported (firefox_driver.mjs; Safari's: none), taken per shown example like ConsoleWatch's"""

	def take(self):
		time.sleep(CONSOLE_SETTLE_SECONDS)
		return list(dict.fromkeys(console_line(message) for message in webdriver.command("messages") if reported(message)))

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


def run_code(code):
	"""the playground's value once it ran `code`"""
	script = f"""(async () => {{
		await playground.runCode({json.dumps(code)});
		while (document.getElementById("status").textContent === "running…") await new Promise(done => setTimeout(done, 50));
		return document.getElementById("value").textContent;
	}})()"""
	shown = browser("eval", script)
	return json.loads(shown) if shown.startswith('"') else f"(page gave no answer: {shown or browser_complaint})"


def text_hole_wrong(code=TEXT_HOLE_CODE, root="section", expected_text=TEXT_HOLE_SHOWN):
	"""clicks on the last button change the text after it (playground.js showText), the elements around it stay"""
	run_code(code)
	shown = browser("eval", f"""(async () => {{
		const root = document.getElementById("rendered").shadowRoot.querySelector("{root}");
		const elements = [root, ...root.querySelectorAll("*")];
		for (let click = 0; click < {TEXT_HOLE_CLICKS}; click++) {{
			[...root.querySelectorAll("button")].at(-1).click();
			await new Promise(done => setTimeout(done, {CLICK_MILLISECONDS}));
		}}
		return JSON.stringify({{ text: root.textContent, kept: elements.every(element => element.isConnected) }});
	}})()""")
	shown = json.loads(json.loads(shown)) if shown.startswith('"') else {"text": f"(page gave no answer: {shown or browser_complaint})"}
	return [f"{part}: {shown.get(part)!r}, expected {expected!r}" for part, expected in (("text", expected_text), ("kept", True)) if shown.get(part) != expected]


def report_wrong(page_url):
	"""?report turns the report button on in this browser and ?report=off off again; its issue holds the program"""
	with_parameter = lambda parameter: f"{page_url}{'&' if '?' in page_url else '?'}{parameter}"
	open_page(with_parameter("report"))
	wait_for_isolation()
	# running waits until the page's worker started its task Workers (host-tasks.js taskPoolReady): a page left before
	# aborts their script loads, which Firefox logs as console errors (NS_BINDING_ABORTED for task-worker.js)
	run_code(REPORT_CODE)
	shown = browser("eval", '!document.getElementById("report").hidden')
	address = json.loads(browser("eval", "window.playground.reportAddress()") or '""')
	open_page(with_parameter("report=off"))
	wait_for_isolation()
	run_code(REPORT_CODE)
	hidden = browser("eval", 'document.getElementById("report").hidden')
	open_page(page_url)  # last: its reloads to other addresses come after the checks that reload the page themselves
	wait_for_isolation()
	wrong = [] if shown == "true" else [f"report button hidden after ?report: {shown}"]
	wrong += [] if hidden == "true" else [f"report button shown after ?report=off: {hidden}"]
	body = urllib.parse.parse_qs(urllib.parse.urlparse(address).query).get("body", [""])[0]
	wrong += [] if REPORT_PRINTED in body.partition("Output:")[2] else [f"report issue without the program's output: {address!r}"]
	return wrong + ([] if REPORT_CODE in body else [f"report issue without the program: {address!r}"])


def file_kept_wrong(page_url):
	"""a file one run wrote is read by a run after the page reloaded: what differs"""
	run_code(WRITE_FILE)
	open_page(page_url)
	wait_for_isolation()
	read = run_code(READ_FILE)
	return [] if FILE_KEPT_TEXT in read else [f"read after a reload: {read!r}, expected {FILE_KEPT_TEXT!r}"]


def stop_button_wrong():
	"""the Run button of a program still running after STOP_GRACE_MS says Stop; pressing it stops the run, and it says Run again"""
	shown = browser("eval", f"""(async () => {{
		const pause = milliseconds => new Promise(done => setTimeout(done, milliseconds));
		const finished = playground.runCode({json.dumps(STOP_BUTTON_CODE)});
		await pause({STOP_BUTTON_WAIT_MS});
		const running = document.getElementById("run").textContent;
		document.getElementById("run").click();
		await finished;
		return JSON.stringify({{ running, after: document.getElementById("run").textContent, value: document.getElementById("value").textContent }});
	}})()""")
	shown = json.loads(json.loads(shown)) if shown.startswith('"') else {"running": f"(page gave no answer: {shown or browser_complaint})"}
	expected = {"running": "Stop ■", "after": "Run ▶", "value": "stopped"}
	return [f"{part}: {shown.get(part)!r}, expected {value!r}" for part, value in expected.items() if shown.get(part) != value]


def member_completion_wrong():
	"""Tab after `xs.` of a list lists its members, which come from the compiler's worker: the list shows them"""
	shown = browser("eval", f"""(async () => {{
		const editor = document.querySelector(".CodeMirror").CodeMirror;
		// unfocused: the assistant's pause timer, left by the Enter of the checks before, continues only a focused editor
		// (assistant.js completeAfterPause), which would load Cloudflare's bot check into the checks after this one
		editor.getInputField().blur();
		await new Promise(done => setTimeout(done, {ASSISTANT_PAUSE_MS})); // that timer passes while unfocused
		editor.setValue({json.dumps(MEMBER_CODE + ".")}); // not typed (replaceSelection is "+input"): no assistant call
		editor.setCursor(0, {len(MEMBER_CODE) + 1});
		editor.triggerOnKeyDown({{ type: "keydown", keyCode: {KEY_CODES["Tab"]}, preventDefault() {{}}, stopPropagation() {{}} }});
		const list = document.querySelector(".completions");
		const deadline = Date.now() + {MEMBERS_WAIT_MS};
		while ((!list || list.hidden) && Date.now() < deadline) await new Promise(done => setTimeout(done, 100));
		return list && !list.hidden ? [...list.children].map(item => item.textContent).join(" ") : "no list";
	}})()""")
	listed = json.loads(shown) if shown.startswith('"') else shown or browser_complaint
	missing = [name for name in MEMBERS_LISTED if name not in listed.split()]
	return [f"{MEMBER_CODE}. then Tab listed {listed!r}, missing {missing}"] if missing else []


def page_elements_wrong():
	"""a click's handler reads what was typed into the input of id who as `$who.value`: its Worker asks the page"""
	run_code(PAGE_ELEMENTS_CODE)
	shown = browser("eval", f"""(async () => {{
		const root = document.getElementById("rendered").shadowRoot;
		root.getElementById("who").value = {json.dumps(PAGE_ELEMENTS_NAME)};
		root.querySelector("button").click();
		await new Promise(done => setTimeout(done, {CLICK_MILLISECONDS}));
		return root.querySelector("p").textContent;
	}})()""")
	shown = json.loads(shown) if shown.startswith('"') else f"(page gave no answer: {shown or browser_complaint})"
	return [] if shown == PAGE_ELEMENTS_SHOWN else [f"typed {PAGE_ELEMENTS_NAME!r} and clicked: shown {shown!r}, expected {PAGE_ELEMENTS_SHOWN!r}"]


def clipboard_read_wrong():
	"""a program reads what the page copied; skipped loudly when the browser refuses the page its own clipboard"""
	refused = browser("eval", f"""(async () => {{
		try {{ await navigator.clipboard.writeText({json.dumps(CLIPBOARD_TEXT)}); await navigator.clipboard.readText(); return ""; }}
		catch (failure) {{ return failure.message; }}
	}})()""")
	if refused not in ('""', ""):
		# card dir-clipboard: the run asks the user with a button instead of failing (a script's click is no user action)
		asked = browser("eval", """(async () => {
			playground.runCode("clipboard");
			for (let wait = 0; wait < 100 && !document.querySelector(".permission-ask"); wait++) await new Promise(done => setTimeout(done, 50));
			const asking = document.querySelector(".permission-ask");
			asking?.remove(); // it would cover the output the later checks click
			return asking ? asking.textContent : document.getElementById("value").textContent;
		})()""")
		print(f"skip reading {CLIPBOARD_READ}: this browser refuses the page its clipboard: {refused}")
		run_code("1")  # the asking run gives up once a new run replaces its worker
		return [] if "click to allow" in asked else [f"a refused clipboard read asks no permission: {asked!r}"]
	read = run_code("clipboard")
	return [] if CLIPBOARD_TEXT in read else [f"clipboard read: {read!r}, expected {CLIPBOARD_TEXT!r}"]


def red_shader_shown():
	"""the value of painting RED_SHADER and how its canvas shows: drawn by the GPU, its red, why the page canvas failed"""
	painted = run_code(RED_SHADER)
	return painted, browser("eval", f"""(async () => {{
		await new Promise(done => setTimeout(done, {CANVAS_SETTLE_MS}));
		const canvas = [...document.querySelectorAll("#paintings canvas")].at(-1);
		if (!canvas) return "no canvas";
		const copy = new OffscreenCanvas(canvas.width, canvas.height).getContext("2d");
		copy.drawImage(canvas, 0, 0);
		const failed = [...document.querySelectorAll("#diagnostics li")].map(item => item.textContent).find(text => text.includes({json.dumps(CANVAS_FAILED)}));
		return `gpu ${{canvas.gpu === true}} red ${{copy.getImageData(4, 4, 1, 1).data[0]}} scaled ${{canvas.style.imageRendering}} ${{failed ?? ""}}`;
	}})()""")


def gpu_page_wrong():
	"""named arrays come back by name, and a painted shader shows red in the GPU's own canvas; skipped loudly without
	WebGPU (the runner's Firefox)"""
	computed = run_code(NAMED_ARRAYS)
	if NO_GPU in computed:
		print(f"skip {GPU_PAGE}: this browser {NO_GPU}")
		return []
	wrong = [f"named arrays: {computed!r}, expected {shown!r} in it" for shown in NAMED_ARRAYS_SHOWN if shown not in computed]
	painted, shown = red_shader_shown()
	if CANVAS_FAILED in shown:
		print(f"skip {GPU_PAGE} (its page canvas): {shown}")
	expected = "red 255" if CANVAS_FAILED in shown else "gpu true red 255"
	wrong += [] if SHADER_SCALED in shown else [f"painted shader: {shown!r}, expected {SHADER_SCALED!r}: a shader's render scales smoothly"]
	return wrong + ([] if expected in shown else [f"painted shader ({painted!r}): {shown!r}, expected {expected!r}"])


def shader_error_wrong():
	"""the shader's error names the line and column in the program and marks the editor there; skipped loudly without
	WebGPU"""
	shown = run_code(SHADER_ERROR_CODE)
	if NO_GPU in shown:
		print(f"skip {SHADER_ERROR}: this browser {NO_GPU}")
		return []
	marked = browser("eval", '[...document.querySelectorAll(".marked-error")].map(mark => mark.textContent).join(" ")')
	wrong = [] if SHADER_ERROR_PLACE in shown else [f"shader error: {shown!r}, expected {SHADER_ERROR_PLACE!r} in it"]
	wrong += [f"shader error: {shown!r}, expected one line without the GPU's excerpt"] if SHADER_EXCERPT in shown else []
	return wrong + ([] if ":" in marked else [f"shader error marks {marked!r}, expected the colon"])


def previous_frame_wrong():
	"""a shader reading $previous adds to what its last render left; skipped loudly without WebGPU"""
	rendered = run_code(PREVIOUS_FRAME_CODE)
	if NO_GPU in rendered:
		print(f"skip {PREVIOUS_FRAME}: this browser {NO_GPU}")
		return []
	return [] if PREVIOUS_FRAME_SHOWN in rendered else [f"third render: {rendered!r}, expected {PREVIOUS_FRAME_SHOWN} in it"]


def view_steady_wrong():
	"""runs painting at the view's size, one after the other, see the same view once the first painting is shown"""
	shown = [run_code(VIEW_STEADY_CODE) for _ in range(VIEW_STEADY_RUNS)]
	return [] if len(set(shown[1:])) == 1 and re.fullmatch(r"\[\d+ \d+\]", shown[-1]) else [f"the view's size, run after run: {shown}, expected the same after the first"]


def gpu_jobs_wrong():
	"""the last of many GPU jobs gives its numbers, or says why not, but never ran out of memory"""
	shown = run_code(GPU_JOBS_CODE)
	return [f"after 400 GPU jobs: {shown!r}"] if GPU_JOBS_NEVER in shown or not shown else []


def many_tasks_wrong():
	"""400 tasks awaited one after the other give their sum"""
	shown = run_code(MANY_TASKS_CODE)
	return [] if shown == MANY_TASKS_SHOWN else [f"after 400 tasks: {shown!r}, expected {MANY_TASKS_SHOWN}"]


def brush_wrong():
	"""the pointer moved to the middle of the shader brush's canvas: the pixel there is colored, not paper. Its
	`while window_open` loop runs only once the page was clicked (playground.js tellSystemValues), so a real click first;
	the sample then runs until stopped, so it is started, not waited for, and checked last (a check after it found
	the page still busy with it)"""
	if not webdriver:  # Firefox has no WebGPU here: the sample's error is the skip below
		browser("click", "#output")
	browser("eval", f"void playground.chooseExample({json.dumps(BRUSH_SAMPLE)})")
	shown = browser("eval", f"""(async () => {{
		const pixel = (canvas, x, y) => {{
			const copy = new OffscreenCanvas(canvas.width, canvas.height).getContext("2d");
			copy.drawImage(canvas, 0, 0);
			return [...copy.getImageData(x, y, 1, 1).data.slice(0, 3)];
		}};
		const page = () => `value ${{document.getElementById("value").textContent}} status ${{document.getElementById("status").textContent}}`;
		for (let waited = 0; waited < {BRUSH_WAIT_MS}; waited += {BRUSH_POLL_MS}) {{
			const canvas = [...document.querySelectorAll("#paintings canvas")].at(-1);
			if (canvas) {{
				const bounds = canvas.getBoundingClientRect();
				canvas.dispatchEvent(new PointerEvent("pointermove", {{ clientX: bounds.left + bounds.width / 2, clientY: bounds.top + bounds.height / 2, bubbles: true }}));
				const middle = pixel(canvas, canvas.width >> 1, canvas.height >> 1);
				if (Math.min(...middle) < {PAPER_FROM}) return "colored";
				if (waited + {BRUSH_POLL_MS} >= {BRUSH_WAIT_MS}) return `middle ${{middle}} corner ${{pixel(canvas, 0, 0)}} gpu ${{canvas.gpu === true}} ${{canvas.width}}×${{canvas.height}} ${{page()}}`;
			}}
			await new Promise(done => setTimeout(done, {BRUSH_POLL_MS}));
		}}
		return `no canvas ${{page()}}`;
	}})()""")
	if NO_GPU in shown:
		print(f"skip {BRUSH}: this browser {NO_GPU}")
		return []
	return [] if "colored" in shown else [f"the middle of the canvas under the pointer: {shown!r}, expected a color, not paper"]


def fingers_wrong():
	"""a touch drag over finger paint's canvas leaves color at its middle; skipped loudly where the browser has no
	DevTools protocol to send touches through (Firefox, Safari)"""
	if webdriver:
		print(f"skip {FINGERS}: touches go through Chrome's DevTools protocol only")
		return []
	browser("click", "#output")
	browser("eval", f"void playground.chooseExample({json.dumps(FINGERS_SAMPLE)})")
	pixel_painted = lambda share: browser("eval", f"""(async () => {{
		const shown = () => [...document.querySelectorAll("#paintings canvas")].at(-1);
		for (let waited = 0; waited < {FINGERS_WAIT_MS}; waited += 100) {{
			const canvas = shown();
			// read from a copy: reading the shown canvas again and again makes Chrome warn on the console
			const copy = canvas && new OffscreenCanvas(canvas.width, canvas.height).getContext("2d");
			copy?.drawImage(canvas, 0, 0);
			const middle = copy?.getImageData(Math.floor(canvas.width * {share}), Math.floor(canvas.height * {share}), 1, 1).data;
			if (middle && Math.min(...middle.slice(0, 3)) < {PAPER_FROM}) return "painted";
			await new Promise(done => setTimeout(done, 100));
		}}
		return `${{shown() ? "paper" : "no canvas"}}, value ${{document.getElementById("value").textContent}} status ${{document.getElementById("status").textContent}} at ${{location.hash}}`;
	}})()""")
	drag = lambda share: subprocess.run(["node", FINGER_DRAG, browser("get", "cdp-url"), *map(str, share)], capture_output=True, text=True, timeout=60).stdout.strip()
	touch_action = lambda value: browser("eval", f'[...document.querySelectorAll("#paintings canvas")].at(-1).style.touchAction = "{value}"')
	pixel_painted(0)  # the canvas is shown
	dragged = drag(FINGERS_PAINTED)
	shown = pixel_painted(sum(FINGERS_PAINTED) / 2)
	touch_action("auto")
	print(f"{FINGERS}: with the browser's own touch handling, {drag(FINGERS_BROWSER_TOUCH)}: {pixel_painted(sum(FINGERS_BROWSER_TOUCH) / 2)}")
	return [] if "painted" in shown else [f"a finger {dragged} over samples/{FINGERS_SAMPLE}: its middle shows {shown}, expected a stroke"]


def frame_shaders_wrong():
	"""each `on frame` shader sample still paints a few seconds in: its canvas changes between two looks; skipped loudly
	where shaders don't paint into the page's canvas (the bug was there), whose 1280×800 pixel frames keep the page
	too busy to answer"""
	painted, shown = red_shader_shown()
	if NO_GPU in painted or "gpu true" not in shown:
		print(f"skip {FRAME_SHADERS}: this browser paints shaders {THROUGH_PIXELS} ({painted!r}, {shown!r})")
		return []
	wrong = []
	for sample in FRAME_SHADER_SAMPLES:
		browser("eval", f"void playground.chooseExample({json.dumps(sample)})")
		shown = browser("eval", f"""(async () => {{
			const page = () => `value ${{document.getElementById("value").textContent}} status ${{document.getElementById("status").textContent}} output ${{document.getElementById("output").textContent.slice(0, 300)}}`;
			const looks = [];
			const shown = () => [...document.querySelectorAll("#paintings canvas")].at(-1);
			for (const started = performance.now(); !shown() && performance.now() - started < {FRAME_SHADER_FIRST_MS};) await new Promise(done => setTimeout(done, 100));
			for (const wait of [0, {FRAME_SHADER_LOOK_AGAIN_MS}]) {{
				for (let moved = 0, started = performance.now(); performance.now() - started < wait; moved++) {{
					const bounds = shown()?.getBoundingClientRect();
					if (bounds) shown().dispatchEvent(new PointerEvent("pointermove", {{ clientX: bounds.left + bounds.width * (0.2 + 0.6 * (moved % 7) / 7), clientY: bounds.top + bounds.height / 2, bubbles: true }}));
					await new Promise(done => setTimeout(done, 100));
				}}
				const now = shown();
				if (!now) {{ looks.push("no canvas"); continue; }}
				const copy = new OffscreenCanvas(now.width, now.height).getContext("2d");
				copy.drawImage(now, 0, 0);
				const data = copy.getImageData(0, 0, now.width, now.height).data;
				let sum = 0;
				for (let at = 0; at < data.length; at += 4093) sum = (sum * 31 + data[at]) >>> 0;
				looks.push(`${{now.width}}×${{now.height}} #${{sum}}`);
			}}
			return looks[0] !== looks[1] && !looks.includes("no canvas") ? "painting" : `looks ${{looks.join(" then ")}}; ${{page()}}`;
		}})()""")
		if NO_GPU in shown:
			print(f"skip {FRAME_SHADERS} {sample}: this browser {NO_GPU}")
		elif "painting" not in shown:
			wrong.append(f"samples/{sample} stopped painting: {shown or 'the check gave no answer'}")
	return wrong


def completion_keys_wrong():
	"""the editor's code after each of COMPLETIONS' keys, pressed through CodeMirror's own key handling: what differs"""
	script = f"""(async () => {{
		const editor = document.querySelector(".CodeMirror").CodeMirror;
		const press = code => editor.triggerOnKeyDown({{ type: "keydown", keyCode: code, preventDefault() {{}}, stopPropagation() {{}} }});
		editor.focus();
		const shown = JSON.stringify({json.dumps(COMPLETIONS)}.map(([code, keys]) => {{
			editor.setValue(code);
			editor.setCursor(editor.lastLine(), editor.getLine(editor.lastLine()).length);
			keys.forEach(key => press({json.dumps(KEY_CODES)}[key]));
			return editor.getValue();
		}}));
		// the Enters left the assistant's pause timer, which continues a focused editor (assistant.js completeAfterPause)
		// and loads Cloudflare's bot check, an "uncaught exception: undefined" in Firefox, into the next check: unfocused,
		// it passes doing nothing
		editor.getInputField().blur();
		await new Promise(done => setTimeout(done, {ASSISTANT_PAUSE_MS}));
		return shown;
	}})()"""
	shown = browser("eval", script)
	if not shown.startswith('"'):
		return [f"(page gave no answer: {shown or browser_complaint})"]
	results = json.loads(json.loads(shown))
	return [f"{code!r} then {' '.join(keys)}: {result!r}, expected {expected!r}" for (code, keys, expected), result in zip(COMPLETIONS, results) if result != expected]


def show_example(name):
	"""the playground's value and printed text once it showed the example, and its timers ran `wait` milliseconds; with
	`typed` (into the first input) and `clicks` also the value after typing and clicking those buttons or links, whether every element shown stayed (`kept`), kept its key (`keyed`) and whether anything animated (`animated`) and the address bar's path (`address`)"""
	script = f"""(async () => {{
		const name = {json.dumps(name)};
		{RUN_EXAMPLE % json.dumps(name)}
		await new Promise(done => setTimeout(done, EXAMPLES[name].wait ?? 0));
		const shown = {{ value: document.getElementById("value").textContent, printed: document.getElementById("printed").textContent,
			canvases: document.querySelectorAll("#paintings canvas").length,
			failed: document.getElementById("status").classList.contains("failed") ? document.getElementById("status").textContent : "" }};
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
	return json.loads(json.loads(shown)) if shown.startswith('"') else {"value": f"(page gave no answer: {shown or browser_complaint})", "printed": ""}


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
	console = WebDriverConsole() if webdriver else ConsoleWatch()  # before the page, so its loading is watched too
	page_url = f"{page_url}{'&' if '?' in page_url else '?'}slow_start={SLOW_START_MS}"
	open_page(page_url)
	wait_for_isolation()
	examples = json.loads(json.loads(browser("eval", "JSON.stringify(EXAMPLES)")))
	samples = json.loads(json.loads(browser("eval", "JSON.stringify(Object.keys(SAMPLES).sort())")))
	failures = []
	def verdict(name, wrong):
		expected = EXPECTED_CONSOLE.get(name, [])
		logged = console.take()
		wrong += [f"console {message}" for message in logged if not any(line in message for line in expected)]
		known = [message for message in logged if any(line in message for line in expected)]
		print(f"{'FAIL' if wrong else 'ok  '} {name}" + "".join(f"\n     {line}" for line in wrong) + "".join(f"\n     known, expected: {line[:200]}" for line in known))
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
			if NO_GPU in str(shown.get("value")):
				print(f"skip {name}: this browser {NO_GPU}")
				shown = {**shown, "value": expected.get("value"), "canvases": expected.get("canvases")}
			verdict(name, [f"{part}: {shown.get(part)!r}, expected {expected[part]!r}" for part in ("value", "printed", "canvases", "clicked", "clickedPrinted", "kept", "keyed", "animated", "address") if part in expected and shown.get(part) != expected[part]] + [f"status: {shown.get('failed')}"] * bool(shown.get("failed")))
	extra_checks = [(check, wrong) for check, wrong in ((COMPLETION_KEYS, completion_keys_wrong), (STOP_BUTTON, stop_button_wrong), (MEMBER_COMPLETION, member_completion_wrong), (CLIPBOARD_READ, clipboard_read_wrong), (PAGE_ELEMENTS, page_elements_wrong), (GPU_PAGE, gpu_page_wrong), (VIEW_STEADY, view_steady_wrong), (GPU_JOBS, gpu_jobs_wrong), (SHADER_ERROR, shader_error_wrong), (MANY_TASKS, many_tasks_wrong), (PREVIOUS_FRAME, previous_frame_wrong), (TEXT_HOLE, text_hole_wrong), (COMPONENT_HOLE, lambda: text_hole_wrong(COMPONENT_HOLE_CODE, "div", COMPONENT_HOLE_SHOWN)), (FILE_KEPT, lambda: file_kept_wrong(page_url)), (BRUSH, brush_wrong), (FINGERS, fingers_wrong), (FRAME_SHADERS, frame_shaders_wrong), (REPORT, lambda: report_wrong(page_url))) if not names or check in names]
	for check, wrong in extra_checks:
		verdict(check, wrong())
	console.stop()
	browser("close")
	if server:
		server.shutdown()
		if failures:
			show_served()
	checked = len(chosen) + 1 + len(extra_checks)
	print(f"\nexamples and samples: {checked - len(failures)} of {checked} show what they promise, without console errors" + (f"; failed: {', '.join(failures)}" if failures else ""))
	sys.exit(101 if failures else 0)


def build_components():
	"""the components `use wasm` tests call, transpiled for the page (components.js); a failure only warns: those tests
	then fail naming build.sh. COMPONENTS_BUILT: built elsewhere already (scripts/browser-dry-run.sh builds them on the
	Mac, the server has no cargo)"""
	if COMPONENTS_BUILT:
		return
	built = subprocess.run([os.path.join(REPOSITORY, "web", "playground", "build.sh"), "components"], capture_output=True, text=True)
	if built.returncode:
		print(f"warning: build.sh components failed, `use wasm` tests will fail:\n{built.stderr.strip()}", file=sys.stderr)


WEBDRIVERS = {"--firefox": FirefoxDriver, "--safari": SafariDriver}


def chrome_allowed():
	"""Chrome runs only in CI (user, 2026-10-09: no test launches Chrome or Chromium on the Mac): --serve starts none,
	--firefox runs Firefox, --safari Safari"""
	return bool(os.environ.get("CI")) or sys.argv[1:2] == ["--serve"] or any(name in sys.argv[2:] for name in WEBDRIVERS)


def live_differences(url, site):
	"""the built files at the top of `site` that `url` serves otherwise, or not at all"""
	def served(name):
		try:
			with urllib.request.urlopen(urllib.parse.urljoin(url, name), timeout=30) as reply:
				return reply.read()
		except OSError as failure:
			return failure
	built = sorted(name for name in os.listdir(site) if os.path.isfile(os.path.join(site, name)))
	return [name for name in built if served(name) != open(os.path.join(site, name), "rb").read()]


def wait_until_deployed(url, site):
	"""waits until `url` serves the files built into `site`; fails naming those still old after DEPLOYED_WAIT_SECONDS"""
	deadline = time.time() + DEPLOYED_WAIT_SECONDS
	while differing := live_differences(url, site):
		if time.time() > deadline:
			sys.exit(f"{url} still serves other files than the build after {DEPLOYED_WAIT_SECONDS} s: {', '.join(differing)}")
		print(f"     waiting for the deploy: {url} serves {len(differing)} files otherwise ({', '.join(differing[:5])})")
		time.sleep(DEPLOYED_POLL_SECONDS)
	print(f"     {url} serves the built files")


def main():
	# each verdict shows in a CI log as it comes, so a stuck run shows where it stuck (card deploy-firefox-hang)
	sys.stdout.reconfigure(line_buffering=True)
	if not chrome_allowed():
		print(CI_ONLY)
		return
	if sys.argv[1:2] == ["--examples"]:
		global webdriver
		names = [name for name in sys.argv[2:] if name not in WEBDRIVERS]
		chosen = [WEBDRIVERS[name] for name in sys.argv[2:] if name in WEBDRIVERS]
		webdriver = chosen[0]() if chosen else None
		place = {}
		while names[:1] in (["--url"], ["--site"]):
			place[names[0]], names = names[1], names[2:]
		if "--url" in place and "--site" in place:
			wait_until_deployed(place["--url"], place.pop("--site"))
		check_examples(names, place.get("--url"), place.get("--site"))
	if sys.argv[1:2] == ["--serve"]:
		binary = os.path.abspath(sys.argv[2]) if len(sys.argv) > 2 else None
		serve(binary)
		tests = f"tests.html?wasm={BINARY_PATH}" if binary else "tests.html (after copying the binary to web/playground/tests.wasm)"
		print(f"serving {REPOSITORY}\n  playground: http://127.0.0.1:{PORT}/web/playground/\n  tests:      http://127.0.0.1:{PORT}/web/playground/{tests}")
		threading.Event().wait()
	binary, arguments = sys.argv[1], [argument for argument in sys.argv[2:] if not argument.startswith(IGNORED_ARGUMENTS)]
	build_components()
	server = serve(binary)
	query = urllib.parse.urlencode({"wasm": BINARY_PATH, "args": json.dumps(arguments), "workers": WORKERS, **({"perWorker": PER_WORKER} if PER_WORKER else {}), **({"shard": SHARD} if SHARD else {})})
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
	if summary.get("timeoutRetries"):
		retried = ", ".join(summary["timeoutRetries"])
		print(f"\n{len(summary['timeoutRetries'])} tests timed out once and ran again on a fresh worker: {retried}")
		if os.environ.get("GITHUB_ACTIONS"):
			print(f"::warning title=A browser test timed out once (card find-why)::{retried}")
	verdict = "FAILED" if summary["failed"] else "ok"
	print(f"\ntest result: {verdict}. {summary['passed']} passed; {len(summary['failed'])} failed; {summary['ignored']} ignored; "
		f"finished in {summary['seconds']:.2f}s (in the browser)")
	sys.exit(101 if summary["failed"] else 0)


if __name__ == "__main__":
	main()
