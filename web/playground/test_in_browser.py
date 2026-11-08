#!/usr/bin/env python3
"""The playground checked in a headless browser, the repository root served.
`test_in_browser.py --examples [name…]` runs the tour (examples.js) in the playground page itself, built by build.sh:
each example's value and printed output must be the ones it names (a broken example fails CI, .github/workflows/pages.yml).
`test_in_browser.py --examples --url https://warp.pannous.com/ [--site _site] [name…]` checks the tour of a deployed
playground instead, with --site once it serves those built files.
`test_in_browser.py --examples --firefox [--url …|--site …] [name…]` runs the same check in headless Firefox
(firefox_driver.mjs, WebDriver BiDi) instead of agent-browser's Chrome; `--safari` in Safari (safaridriver, classic
WebDriver: macOS CI runners, after `sudo safaridriver --enable`; its console is not watched).
`test_in_browser.py --examples --guide` checks the guide instead (guide.md: each snippet's `=> value` and ```printed
lines); with example names, both.
`test_in_browser.py --examples --site _site [name…]` checks it in a collected site (pages.yml), served as its root: a file
the deploy forgot fails here, not on the live page.
`test_in_browser.py --samples [samples/….warp…]` (scripts/samples-web.sh) runs every sample in the page, each as its own
file, and compares its output with the native run's (scripts/samples.sh: data/samples/out); writes test/samples-web.txt.
`test_in_browser.py --tests [test/<topic>…]` (scripts/tests-web.sh) runs every case of test/ (each pair of a *.pairs.warp,
each is! line) in the page, judged as the native `warp test` judges (extensions/commands/tests.warp: warp.wasm lists the
cases, the page runs them, warp.wasm gives the verdicts), against the native verdicts of the stored baseline
(scripts/baseline.sh); writes test/tests-web.txt.
`test_in_browser.py --serve` only serves (http://127.0.0.1:PORT/web/playground/), for any browser.
Besides the repository it serves /__stub__?status=…&body=… (that response, for fetch tests) and /__include__/<header>: the C header of that name from the first include directory of
src/formats/ffi_parser.rs INCLUDE_DIRS that holds it (the page sets WARP_INCLUDE=/include), nothing else of the machine."""
import functools, http.server, json, os, re, subprocess, sys, threading, time, urllib.parse, urllib.request

# --samples (scripts/samples-web.sh): every samples/**/*.warp in the page, its output against the native run's
SAMPLES_WEB_RESULTS = "test/samples-web.txt"  # tracked: status and sample, one a line, like test/samples.txt
# a sample that fails on purpose says why on a line of its own, `# expected failure: deliberate parse errors`: EXP
# instead of FAIL in either sweep (scripts/samples.sh EXPECTED_FAILURE); in the page ok when it fails with the native
# run's error, it gives what the native run gives (decision exp-counts)
EXPECTED_FAILURE = re.compile(r"^# expected failure: ", re.M)
# where a sample runs, a line of its own: `# host: native` or `# host: none` stay out of the page's sweep and menu
# (build.sh), `# host: page` runs here with no native output to compare (scripts/samples.sh HOST_LINE)
HOST_LINE = re.compile(r"^# host: ([a-z]+)", re.M)
def sample_host(source):
	"""where a sample says it runs (HOST_LINE): native, page, none, or both when it says nothing"""
	host = HOST_LINE.search(source)
	return host.group(1) if host else "both"
# a sample's interactions (card scripts-samples, decision sample-checks): `# check: click "+1", type "name" "Ann" → text`,
# each from a fresh run, its steps done on the page, its text then holding the text after → (natively `warp app check`)
CHECK_LINE = re.compile(r"^# check: (.*) → (.*)$", re.M)
CHECK_STEP = re.compile(r'(click|type)((?:\s+"[^"]*")+)')
NATIVE_RESULTS = "test/samples.txt"  # scripts/samples.sh: WIN samples need a C window library, never run in a page
# scripts/samples.sh: <sample with / as _>.txt, what the native run printed (samples-web.sh names the main checkout's)
NATIVE_OUTPUTS = os.environ.get("WARP_NATIVE_OUTPUTS", "data/samples/out")
SAMPLES_WEB_DATA = "data/samples-web"  # report.txt (each difference's first lines) and out/, what the page printed
# the page stops a run after this (playground.js ?run_seconds, natively scripts/samples.sh gives 30 s) and answers; a
# page that has not answered 10 s later is stuck
SAMPLE_RUN_SECONDS = 30
SAMPLE_ANSWER_SECONDS = SAMPLE_RUN_SECONDS + 10
SHOWN_DIFFERENCE = 300  # characters of each side in report.txt
# a duration a sample measures (`painted in 53 ms`) differs on every run: both sides compare with it masked
MEASURED_TIME = re.compile(r"\b\d+(\.\d+)? ?(ms|µs|ns)\b")
# --tests (scripts/tests-web.sh): every case of test/ in the page, its verdict against the native baseline's
TESTS_WEB_RESULTS = "test/tests-web.txt"  # tracked: status and case, one a line
TESTS_WEB_DATA = "data/tests-web"  # work/ (the native test work folder, its runs' results written by the page), report.txt
PAGE_COMPILER = "web/playground/warp-lib/warp.wasm"  # the page's compiler lists and judges the cases natively too
CASE_RUN_SECONDS = 30  # extensions/commands/tests.warp CASE_SECONDS: the page stops a run after it (?run_seconds)
CASE_ANSWER_SECONDS = CASE_RUN_SECONDS + 10  # a page that finished no case in this long is stuck
CASE_BATCH = 100  # cases the page runs in one go, its reports collected as they finish
CASE_FILE = "test_case.warp"  # a case runs as a file of its test file's folder, so its uses resolve beside it
STOPPED = "stopped after"  # playground.js: the report of a run the page stopped (the native run's `timeout`)
# a section of a test file the browser cannot run (a C window library, a shell command) says why in its heading,
# `# raylib_calls — native only: raylib, a C library the browser has not`: EXP instead of PAGE for the cases below it, up
# to the next blank line (night decision N20, as the samples' EXPECTED_FAILURE; in the heading, as a line of its own
# would move the cases' lines the native verdicts are stored by); a sample with such a line is EXP instead of FAIL
NATIVE_ONLY = re.compile(r"^#.* native only: ", re.M)
CASE_PLACE = re.compile(r"^(\S+):(\d+) ")
PAGE_LOST = "playground is not defined"  # a report of a page that lost its API mid-batch: no verdict, the case runs again
# a test-verdicts line: its mark, then the case; a failure's ends in what it got (scripts/baseline.sh case_of)
VERDICT = re.compile(r"^([✓✗]) (.*?)(?:: got .*)?$")
# a free port per run (0: the system picks one), so runs of several sessions never meet; WARP_BROWSER_TEST_PORT fixes it
PORT = int(os.environ.get("WARP_BROWSER_TEST_PORT", "0"))
# the tour's page starts its worker this late (playground.js ?slow_start): a fast machine then meets the races a slow
# CI runner meets, e.g. "ready" hiding the first example's "running…" (empty value, the failed deploys of 2026-10-08)
SLOW_START_MS = 2000
# one browser per run: runs at the same time (the suite, a tour check) must never drive or close each other's page
SESSION = f"warp-browser-tests-{os.getpid()}"
LAUNCH_ATTEMPTS = 3
LAUNCH_SECONDS = 90  # a page that is not loaded by then did not start: relaunch, loudly
# `--url … --site _site`: the deployed site is checked once it serves the built files, every one of them (card
# playground-verify: right after a deploy the CDN served a mix of old and new files, 58 examples failing)
DEPLOYED_WAIT_SECONDS = 600
DEPLOYED_POLL_SECONDS = 10
INCLUDE_PREFIX = "/__include__/"
# where /__include__/<header> looks (wasi.js WARP_INCLUDE): the Rust compiler's FFI parser read its list from
# src/formats/ffi_parser.rs, gone with src/
HEADER_DIRECTORIES = ("/opt/homebrew/include", "/usr/local/include", "/usr/include")
HEADER_NAME = re.compile(r"^[\w.+-]+(/[\w.+-]+)*\.h$")
LISTING_QUERY = "listing"
STUB_PATH = "/__stub__"  # answers with the status and body its query names, after `delay` milliseconds: the fetch tests' HTTP stub (tests/common serve)
CLICK_MILLISECONDS = 300  # a click's handler runs in the worker and its markup comes back
ISOLATION_SECONDS = 30  # how long a deployed page may take to reload under its service worker
# serve without the isolation headers, as GitHub Pages does, so the page isolates itself under coi-serviceworker.js
# (card firefox-coi-worker)
UNISOLATED = os.environ.get("WARP_SERVE_UNISOLATED") == "1"
REPOSITORY = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
CI_ONLY = "browser tests run in CI only: skipped here, Chrome is not started outside CI (dispatch the Playground workflow on your branch)"
# the console of the page and its workers (console_watch.mjs): any error or warning there fails the tour check
CONSOLE_WATCHER = os.path.join(REPOSITORY, "web", "playground", "console_watch.mjs")
FIREFOX_DRIVER = os.path.join(REPOSITORY, "web", "playground", "firefox_driver.mjs")
webdriver = None  # the FirefoxDriver or SafariDriver of a --firefox or --safari run: browser() and the console go to it instead of agent-browser
SAFARI_PORT = 4445
SAFARI_START_SECONDS = 10
SAFARI_SCRIPT_MS = 300_000  # an example that runs this long is stuck
MEMORY_GROWTHS_SHOWN = 5  # the examples that grew the cgroup's memory most (card our-playground)
# agent-browser's eval for classic WebDriver: the expression's value, awaited, as JSON text ("" for undefined)
SAFARI_EVAL = """const done = arguments[arguments.length - 1];
Promise.resolve().then(() => (0, eval)(arguments[0])).then(value => done(value === undefined ? "" : JSON.stringify(value)), failure => done(`✗ ${failure?.message ?? failure}`));"""
CONSOLE_READY = "ready"
CONSOLE_SETTLE_SECONDS = 0.3  # a shown example's last messages reach the watcher
# warnings the browser logs itself, which no page code can silence, and what the program does instead (card
# browser-tour): Chrome without WebGPU says this on every navigator.gpu.requestAdapter(); @gpu then maps on the CPU and
# says so in the program's own warning, gpu_render fails with the program's error (shown on the page, not the console)
# Chrome's Dawn says the second when the next example ends the task Worker of an animating GPU sample while a frame of
# its OffscreenCanvas is in flight (samples/gpu/manual_3d_object_viewer, cards flaky-viewer, flaky-deploy): the stopped
# program's device is gone, nothing the next example sees
BROWSER_OWN_WARNINGS = {"No available adapters.", "A valid external Instance reference no longer exists."}
# an example on the GPU (paint of a shader) in a browser without WebGPU (the runner's Firefox): skipped, loudly
NO_GPU = "offers no WebGPU adapter"
PAGE_LOAD = "(loading the page)"
# `--examples --guide`: every guide.md snippet's `=> value` and ```printed lines in the page (guide.js SNIPPET_FENCE)
GUIDE = "--guide"
GUIDE_FILE = "web/playground/guide.md"
SNIPPET_FENCE = re.compile(r"^```warp(?: => (.*))?$")
PRINTED_FENCE = "```printed"
# a snippet run in the page (%s: its code as JSON, the milliseconds it may take): its value and stdout, or {stuck}
RUN_SNIPPET = """(async () => {
	const report = await Promise.race([playground.evaluate(%s), new Promise(done => setTimeout(() => done({ stuck: true }), %d))]);
	return JSON.stringify(report.stuck ? report : { value: report.value, printed: (report.printed || []).filter(part => part.stream !== 2).map(part => part.text).join("") });
})()"""
TEXT_HOLE = "(a computed text between children changes alone)"  # card web-fine-text-nodes: markup.rs text_holes
TEXT_HOLE_CODE = 'use web\nn = 0\ndiv{ section{ h1{ "Counter" } button{ on click { n += 1 }, "add" } "clicked " + n + " times" } }'
TEXT_HOLE_CLICKS = 3
TEXT_HOLE_SHOWN = "Counteraddclicked 3 times"
COMPONENT_HOLE = "(a component instance's text changes alone)"  # card web-fine-components: component_state.rs hole_bindings
COMPONENT_HOLE_CODE = 'use web\ndef Tally(start) {\n\tn = start\n\tsection{ button{ on click { n += 1 }, "add" } "clicked " + n + " times" }\n}\ndiv{ Tally(2) Tally(7) }'
COMPONENT_HOLE_SHOWN = "addclicked 2 timesaddclicked 10 times"
REPORT = "(?report shows the report button, which drafts an issue with the program)"  # card report-button
REPORT_CODE = 'print "reported"'
REPORT_PRINTED = "reported"
BUG_BUTTON = "(the bug button sends a comment with the program; ?bug_token stores the token, a wrong one is refused)"  # card bug-report
BUG_TOKEN = "wrong-probe-token"  # never the real one: the endpoint refuses it, so no card is made
BUG_COMMENT = "probe comment"
FIX_BUTTON = "(an \"I meant\" button rewrites the code and runs it again)"  # card clicking-meant: playground.js applyFix
FIX_BUTTON_CODE = "a=5; b=1; a-b:2; a-b"  # the kebab key's fix renames it at the key and at its read (P81)
FIX_BUTTON_LABEL = "I meant: a_b"
FIX_BUTTON_FIXED = ("2", "a=5; b=1; a_b:2; a_b")
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
# card there-should: the clear button beside the bug button empties the editor, its undo brings the program back
CLEAR_BUTTON = "(the clear button empties the editor, undo brings the program back)"
CLEAR_BUTTON_CODE = "x = 2\nx * 3"
MEMBER_COMPLETION = "(members listed after a dot)"  # card member-completion: completion.js asks the compiler (members.rs)
MEMBER_CODE = "xs = [1 2]; xs"
MEMBERS_LISTED = ["count", "sum", "map", "unique"]
MEMBERS_WAIT_MS = 10000
ASSISTANT_PAUSE_MS = 1500  # assistant.js PAUSE_BEFORE_COMPLETION_MS and a margin
PAGE_ELEMENTS = "(a handler reads the value of the page's element of an id: $who)"  # card page-ids: host-foreign.js callOnPage
PAGE_ELEMENTS_CODE = 'use web\ngreeting = "-"\ndiv{ input{ id: "who" } button{ on click { greeting = "Hello " + $who }, "Greet" } p{ greeting } }'
PAGE_ELEMENTS_NAME = "Ada"
PAGE_ELEMENTS_SHOWN = "Hello Ada"
# scripts/build-site.sh: a markup program built as a static site, its page loaded and its button clicked (card
# blockcompiler-entries: built sites on page_render/page_event); checked last, it leaves the playground's page
BUILT_SITE = "(a site scripts/build-site.sh built shows its page and runs its handlers)"
BUILT_SITE_CODE = 'use web\ncount = 0\ndiv{ h1{ "Count " + count } button{ on click { count += 1 }, "add" } }\n'
BUILT_SITE_FOLDER = "data/site-check"
BUILT_SITE_SHOWN = ["Count 0", "Count 1"]  # the heading after main, after the click
BUILT_SITE_WAIT_MS = 20000
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
NAMED_ARRAYS_SHOWN = ["11.5", "6, 7, 8"]
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
# (it did by ~40 px a frame, until Safari ran out of memory painting samples/graphics/webgpu)
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
JOBS times { try gpu_compute(shader, [1], 1) else 0 }
gpu_compute(shader, [1], 1)"""
GPU_JOBS_COUNT = 400  # twice Safari's ~200
# Firefox on lavapipe: 400 jobs outlast RUN_TIMEOUT_MS, and the stopped run's terminate logs Firefox's anonymous
# uncaught exception (card firefox-gpu-jobs-uncaught); 50 finish in time and log nothing (card firefox-tour). Safari's
# bug is Safari's, so Firefox runs the jobs' path with fewer of them
FIREFOX_GPU_JOBS_COUNT = 50
GPU_JOBS_NEVER = "Out of memory"
# the console lines a check makes on purpose: Chrome logs the broken shader's WGSL error itself
# and Firefox its own two lines of the broken shader
# Firefox's bug: terminating a Worker busy inside a WebAssembly.promising call logs an anonymous uncaught exception on
# the page, which no listener sees (card firefox-logs). Stop does that: the
# compiler's web_evaluate and the program's main run under promising (worker.js, host.js JSPI); the run stopped and
# the button right. The clipboard check does the same on purpose: its run_code("1") replaces the worker of the run that
# waits for the permission click (card tour-firefox-fails: two such lines on each Firefox run of the check alone)
FIREFOX_TERMINATED_PROMISING = ["uncaught exception: undefined"]  # card firefox-logs
# samples Firefox's tour does not run (card firefox-tour-fails), each with why: the swirl's run leaves its worker inside
# WebAssembly.promising, so whichever run replaces that worker next logs Firefox's anonymous uncaught exception (dry runs
# 2026-10-15: on the swirl itself, also with a 3 s loop, or on the sample after it); card logs-uncaught finds the cause
FIREFOX_SKIPPED_SAMPLES = {"warp_compiled_wgsl_shader": "its worker, replaced, logs 'uncaught exception: undefined' (card logs-uncaught)"}
# samples no tour runs : the largest growths of the tour's memory
# (card our-playground, isolated runs after 5b0b7f1bdb, anon)
MEMORY_HUNGRY_SAMPLES = {"ringed_planet": "+211 MB: a 1920×1080 compute_toy painted on the CPU in headless Chrome",
	"uniscript_text": "+195 MB: reads uniscript's 10 MB entities index", "sound_patterns": "+100 MB"}
EXPECTED_CONSOLE = {SHADER_ERROR: ["Error while parsing WGSL", "errors while creating shader module", "parsing error"],
	STOP_BUTTON: FIREFOX_TERMINATED_PROMISING, CLIPBOARD_READ: FIREFOX_TERMINATED_PROMISING,
	BUG_BUTTON: ["403"]}  # the browser logs the refused wrong token itself
# card safari-shared-buffers: a task's answer buffer, like a GPU job's, is reused once read
MANY_TASKS = "(many tasks in one run)"
MANY_TASKS_CODE = """twice(x) := x * 2
total = 0
for i in 1 to 400 { total += await go twice(i) }
total"""
MANY_TASKS_SHOWN = "160400"
# card shader-brush: samples/graphics/shader_brush.warp colors the paper under the pointer (its $mouse), not only a corner
BRUSH = "(samples/graphics/shader_brush colors the paper under the pointer)"
BRUSH_SAMPLE = "shader_brush"
BRUSH_WAIT_MS = 8000  # the sample paints a frame every 16 ms; a software GPU takes far longer for its 1280×800
BRUSH_POLL_MS = 250
PAPER_FROM = 240  # a channel this light or lighter on all three: the sample's paper (0.98), no brush
# card register-fingers: a finger dragged over samples/graphics/finger_paint's canvas paints a stroke (the iPad's Safari took the
# touch over for scrolling). Then, only told, a drag over the canvas set back to the browser's touch handling, on a
# part the first stroke didn't reach: whether this browser takes a touch over at all (else the check can't fail); it
# goes last, as Chrome's swipe went back in the page's history, to another example
FINGERS = "(a finger paints in samples/graphics/finger_paint)"
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
# card playground-stale-value: a run started without the Run button (an example chosen, auto-run, runCode) clears the
# last run's value as Run does: an animation running long showed the value before it above itself
STALE_VALUE = "(a run started without Run leaves no old value above an animation)"
STALE_VALUE_CODE = "use draw\ncanvas(40, 40)\nloop {\n\tclear(paper)\n\tshow()\n\tsleep(16ms)\n}"
STALE_VALUE_FIRST = "4242"
STALE_VALUE_WAIT_MS = 3000
# card run-during-load-feedback: while the worker still loads (?slow_start; 7.5 s on the user's iPad), the page's run is
# pending as any run is: Run becomes Stop after the grace, and Stop cancels the run before it starts
STOP_WHILE_LOADING = "(Stop shows while the compiler loads, and cancels the run)"
STOP_WHILE_LOADING_WAIT_MS = 1500  # past STOP_GRACE_MS, before SLOW_START_MS lets the worker start the run
# each GET the server began and finished, (seconds since it started, path, finished): shown after a failed tour, which
# then says whether the browser asked for a stalled worker's scripts at all (card tour-firefox)
served = []
SERVED_SHOWN = 400  # the first requests: the page's and its workers' loading


def find_header(name):
	if not HEADER_NAME.match(name):
		return None
	return next((path for path in (os.path.join(directory, name) for directory in HEADER_DIRECTORIES) if os.path.isfile(path)), None)


def log_served(path, finished):
	served.append((time.time() - SERVER_STARTED, path, finished))


def show_served():
	print(f"the server's first {SERVED_SHOWN} requests (s since it started, → asked, ✓ answered):")
	for seconds, path, finished in served[:SERVED_SHOWN]:
		print(f"  {seconds:7.2f} {'✓' if finished else '→'} {path}")


SERVER_STARTED = time.time()


def serve(root=REPOSITORY):

	class Handler(http.server.SimpleHTTPRequestHandler):
		def translate_path(self, path):
			path = urllib.parse.unquote(urllib.parse.urlsplit(path).path)
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
			if urllib.parse.urlsplit(self.path).path != STUB_PATH:
				return self.send_error(404)
			self.echo()

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
		request_queue_size = 512  # the page's workers fetch their scripts at once
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
# a window sample (WIN) still animating this long after it started (a raylib window's loop, natively until Escape) is
# stopped and judged by the frames it showed (playground.js stopAnimation); other samples run to their end
WINDOW_ANIMATION_SECONDS = 5
SAMPLE_POLL_SECONDS = 0.5
# a tour's sample started, then asked after in short evals (one eval waiting out a window's frames outlasted the
# browser daemon's answer); its animation, which runs until the next run (playground.js startsFrame), is stopped after
# WINDOW_ANIMATION_SECONDS: {running, failed}
CHOOSE_SAMPLE = """window.sampleChosen = false; playground.chooseExample(%s).then(() => window.sampleChosen = true); "started";"""
SAMPLE_STATUS = """(() => { const status = document.getElementById("status");
	const running = !window.sampleChosen || status.textContent === "running…";
	if (running && %s) playground.stopAnimation();
	return JSON.stringify({ running, failed: !running && status.classList.contains("failed") ? status.textContent : "" }); })()"""


# the sample shown anew (%s: its name), each step done on its rendered page as lib/markup.warp's click and type_into
# find an element, by its text, placeholder, aria-label, name or id (%s: the steps as [word, label, text]): the
# page's text after them, or the label no element shows
CHECK_STEPS = """(async () => { """ + RUN_EXAMPLE + """
	const root = document.getElementById("rendered").shadowRoot;
	const labels = element => [element.textContent.trim(), ...["placeholder", "aria-label", "name", "id"].map(name => element.getAttribute(name))];
	for (const [word, label, text] of %s) {
		const element = [...root.querySelectorAll("*")].reverse().find(element => labels(element).includes(label));
		if (!element) return JSON.stringify({ missing: label });
		if (word === "type") {
			element.value = text;
			element.dispatchEvent(new Event("input", { bubbles: true }));
		} else element.click();
		await new Promise(done => setTimeout(done, """ + str(CLICK_MILLISECONDS) + """));
	}
	return JSON.stringify({ text: [...root.children].map(element => element.innerText).join(" ") });
})()"""


def spaced(text):
	"""the text with each run of white space one space, as `warp app check` compares (lib/app/command.warp spaced)"""
	return " ".join(text.split())


def checks_wrong(name, source):
	"""each check line of the sample that the page does not pass, with what it showed"""
	wrong = []
	for steps, expected in CHECK_LINE.findall(source):
		parsed = [[word, *re.findall(r'"([^"]*)"', quoted)] for word, quoted in CHECK_STEP.findall(steps)]
		answer = browser("eval", CHECK_STEPS % (json.dumps(name), json.dumps(parsed)))
		shown = json.loads(json.loads(answer)) if answer.startswith('"') else {"text": f"(page gave no answer: {answer or browser_complaint})"}
		if "missing" in shown:
			wrong.append(f"check {steps} → {expected}: no element shows {shown['missing']}")
		elif spaced(expected) not in spaced(shown["text"]):
			wrong.append(f"check {steps} → {expected}: the page shows {spaced(shown['text'])[:SHOWN_DIFFERENCE]}")
	return wrong


def run_sample(name):
	"""what is wrong with the sample's run: its failed status (`cannot open mozart.mp3`, card open-mozart), as the console
	shows no error for it"""
	browser("eval", CHOOSE_SAMPLE % json.dumps(name))
	started_at = time.time()
	while time.time() < started_at + SAMPLE_ANSWER_SECONDS:
		time.sleep(SAMPLE_POLL_SECONDS)
		answer = browser("eval", SAMPLE_STATUS % json.dumps(time.time() - started_at > WINDOW_ANIMATION_SECONDS))
		if not answer.startswith('"'):
			return [f"status: (page gave no answer: {answer or browser_complaint})"]
		shown = json.loads(json.loads(answer))
		if not shown["running"]:
			return [f"status: {shown['failed']}"] if shown["failed"] else []
	return [f"status: still running after {SAMPLE_ANSWER_SECONDS} s"]


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


# a page script's wait until the run going on is shown
SHOWN_WAIT = 'while (document.getElementById("status").textContent === "running…") await new Promise(done => setTimeout(done, 50));'


def wait_for_first_run():
	"""the page runs its example as it loads: a check run first otherwise met that run, its own queued behind it (card
	stop-button-check-red: Run never became Stop within the check's wait, the worker started ?slow_start late)"""
	browser("eval", f"(async () => {{ {SHOWN_WAIT} return true; }})()")


def run_code(code):
	"""the playground's value once it ran `code`"""
	script = f"""(async () => {{
		await playground.runCode({json.dumps(code)});
		{SHOWN_WAIT}
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


def bug_button_wrong(page_url):
	"""?bug_token=… is stored and leaves the address; the report holds comment, program and output; pannous.com refuses
	the wrong token, which the page then forgets"""
	open_page(f"{page_url}{'&' if '?' in page_url else '?'}bug_token={BUG_TOKEN}")
	wait_for_isolation()
	run_code(REPORT_CODE)
	shown = browser("eval", f"""(async () => {{
		const address = location.href;
		document.getElementById("bug").open = true;
		document.getElementById("bug-comment").value = {json.dumps(BUG_COMMENT)};
		document.getElementById("bug-express").checked = true;
		const report = playground.bugReport();
		document.getElementById("bug-form").requestSubmit();
		const status = document.getElementById("bug-status");
		for (let waited = 0; !status.textContent.startsWith("not sent") && waited < 15000; waited += 100) await new Promise(done => setTimeout(done, 100));
		return JSON.stringify({{ address, report, status: status.textContent, forgotten: localStorage.getItem("warp-playground-bug-token") === null }});
	}})()""")
	shown = json.loads(json.loads(shown)) if shown.startswith('"') else {"report": {}, "status": f"(page gave no answer: {shown or browser_complaint})"}
	open_page(page_url)
	wait_for_isolation()
	report = shown["report"]
	expected = {"token": BUG_TOKEN, "comment": BUG_COMMENT, "code": REPORT_CODE, "express": True}
	wrong = [f"bug report {name}: {report.get(name)!r}, expected {value!r}" for name, value in expected.items() if report.get(name) != value]
	wrong += [] if REPORT_PRINTED in report.get("output", "") else [f"bug report without the program's output: {report.get('output')!r}"]
	wrong += [] if BUG_TOKEN not in shown.get("address", "") else [f"bug token left in the address: {shown['address']}"]
	wrong += [] if "wrong token" in shown["status"] else [f"wrong bug token not refused: {shown['status']!r}"]
	return wrong + ([] if shown.get("forgotten") else ["refused bug token still stored"])


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


def clear_button_wrong():
	"""pressing the clear button leaves an empty editor; the editor's undo shows the program again"""
	shown = browser("eval", f"""(() => {{
		const editor = document.querySelector(".CodeMirror").CodeMirror;
		editor.setValue({json.dumps(CLEAR_BUTTON_CODE)});
		document.getElementById("clear-code").click();
		const cleared = editor.getValue();
		editor.undo();
		return JSON.stringify({{ cleared, undone: editor.getValue() }});
	}})()""")
	shown = json.loads(json.loads(shown)) if shown.startswith('"') else {"cleared": f"(page gave no answer: {shown or browser_complaint})"}
	expected = {"cleared": "", "undone": CLEAR_BUTTON_CODE}
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
	"""a click's handler reads what was typed into the input of id who as `$who`: its Worker asks the page"""
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


def built_site_wrong():
	"""the site's heading after main and after a click on its button"""
	program = os.path.join(REPOSITORY, BUILT_SITE_FOLDER, "app.warp")
	os.makedirs(os.path.dirname(program), exist_ok=True)
	with open(program, "w") as file:
		file.write(BUILT_SITE_CODE)
	built = subprocess.run([os.path.join(REPOSITORY, "scripts", "build-site.sh"), program], capture_output=True, text=True)
	if built.returncode:
		return [f"scripts/build-site.sh failed: {(built.stderr or built.stdout).strip()[-300:]}"]
	browser("open", f"http://127.0.0.1:{PORT}/{BUILT_SITE_FOLDER}/site/index.html")
	shown = browser("eval", f"""(async () => {{
		const heading = async before => {{
			const deadline = Date.now() + {BUILT_SITE_WAIT_MS};
			const text = () => document.querySelector("#warp-root h1")?.textContent;
			while (text() === before && Date.now() < deadline) await new Promise(done => setTimeout(done, 100));
			return text() ?? null;
		}};
		const first = await heading(undefined);
		document.querySelector("#warp-root button")?.click();
		return JSON.stringify([first, await heading(first)]);
	}})()""")
	shown = json.loads(json.loads(shown)) if shown.startswith('"') else f"(page gave no answer: {shown or browser_complaint})"
	return [] if shown == BUILT_SITE_SHOWN else [f"headings {shown!r}, expected {BUILT_SITE_SHOWN!r}"]


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


def stop_while_loading_wrong(page_url):
	"""the page loaded anew: before its worker is ready the Run button says Stop, and pressing it ends the run as stopped"""
	open_page(page_url)
	wait_for_isolation()
	shown = browser("eval", f"""(async () => {{
		const pause = milliseconds => new Promise(done => setTimeout(done, milliseconds));
		const button = document.getElementById("run");
		for (let waited = 0; waited < {STOP_WHILE_LOADING_WAIT_MS} && !button.classList.contains("stop"); waited += 50) await pause(50);
		const loading = button.textContent;
		button.click();
		await pause(300);
		return JSON.stringify({{ loading, after: button.textContent, value: document.getElementById("value").textContent }});
	}})()""")
	wait_for_first_run()
	shown = json.loads(json.loads(shown)) if shown.startswith('"') else {"loading": f"(page gave no answer: {shown or browser_complaint})"}
	expected = {"loading": "Stop ■", "after": "Run ▶", "value": "stopped"}
	return [f"{part}: {shown.get(part)!r}, expected {value!r}" for part, value in expected.items() if shown.get(part) != value]


def stale_value_wrong():
	"""a value shown, then an animation started by runCode: while it runs, the value line no longer shows that value"""
	run_code(STALE_VALUE_FIRST)
	shown = browser("eval", f"""(async () => {{
		playground.runCode({json.dumps(STALE_VALUE_CODE)});
		await new Promise(done => setTimeout(done, {STALE_VALUE_WAIT_MS}));
		const line = document.getElementById("value");
		const value = line.parentElement.hidden ? "" : line.textContent;
		return value;
	}})()""")
	return [f"while the animation runs the value line shows {shown}, the last run's"] if STALE_VALUE_FIRST in shown else []


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
	return [] if len(set(shown[1:])) == 1 and re.fullmatch(r"\[\d+,? \d+\]", shown[-1]) else [f"the view's size, run after run: {shown}, expected the same after the first"]


def gpu_jobs_wrong():
	"""the last of many GPU jobs gives its numbers, or says why not, but never ran out of memory"""
	jobs = FIREFOX_GPU_JOBS_COUNT if isinstance(webdriver, FirefoxDriver) else GPU_JOBS_COUNT
	shown = run_code(GPU_JOBS_CODE.replace("JOBS", str(jobs)))
	return [f"after {jobs} GPU jobs: {shown!r}"] if GPU_JOBS_NEVER in shown or not shown else []


def fix_button_wrong():
	"""the first "I meant" button, clicked as a person clicks it (pointer events, not element.click(): the user's click
	had no effect while the probe's passed), leaves the fixed code and its value"""
	run_code(FIX_BUTTON_CODE)
	found = browser("eval", f"""(() => {{
		const button = [...document.querySelectorAll(".apply-fix")].find(button => button.textContent === {json.dumps(FIX_BUTTON_LABEL)});
		if (button) button.id = "fix-under-test";
		return Boolean(button);
	}})()""")
	if found != "true":
		return [f"no {FIX_BUTTON_LABEL!r} button: {found or browser_complaint}"]
	if webdriver:
		browser("eval", "document.getElementById('fix-under-test').click()")
	else:
		browser("click", "#fix-under-test")
	shown = browser("eval", """(async () => {
		await new Promise(done => setTimeout(done, 100));
		while (document.getElementById("status").textContent === "running…") await new Promise(done => setTimeout(done, 50));
		return JSON.stringify([document.getElementById("value").textContent, playground.code()]);
	})()""")
	shown = tuple(json.loads(json.loads(shown))) if shown.startswith('"') else (f"(page gave no answer: {shown or browser_complaint})",)
	return [] if shown == FIX_BUTTON_FIXED else [f"after clicking the fix: {shown!r}, expected {FIX_BUTTON_FIXED!r}"]


def many_tasks_wrong():
	"""400 tasks awaited one after the other give their sum"""
	shown = run_code(MANY_TASKS_CODE)
	return [] if shown == MANY_TASKS_SHOWN else [f"after 400 tasks: {shown!r}, expected {MANY_TASKS_SHOWN}"]


def brush_wrong():
	"""the pointer moved to the middle of the shader brush's canvas: the pixel there is colored, not paper. Its
	`while window_open` loop runs only once the page was clicked (playground.js tellSystemValues), so a real click first;
	the sample then runs until stopped, so it is started, not waited for, and checked last (a check after it found
	the page still busy with it)"""
	# Firefox's driver clicks for real too, now that its WebGPU paints the brush; Safari's has no click: without WebGPU
	# the sample's error is the skip below
	if not isinstance(webdriver, SafariDriver):
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
		// a page of markup shows its HTML (driver.js page_render), anything else its value
		const shownValue = () => {{ const page = document.getElementById("rendered"); return !page.hidden && page.shadowRoot ? page.shadowRoot.innerHTML : document.getElementById("value").textContent; }};
		const shown = {{ value: shownValue(), printed: document.getElementById("printed").textContent,
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
		return JSON.stringify({{ ...shown, clicked: shownValue(), clickedPrinted, kept, keyed, animated: animations > 0, address: shownAddress() }});
	}})()"""
	shown = browser("eval", script)
	return json.loads(json.loads(shown)) if shown.startswith('"') else {"value": f"(page gave no answer: {shown or browser_complaint})", "printed": ""}


def wait_for_isolation():
	"""a deployed page reloads once under coi-serviceworker.js (index.html) before it is cross-origin isolated: an example
	chosen before that reload is lost; the local server sends the headers itself"""
	deadline = time.time() + ISOLATION_SECONDS
	while browser("eval", "document.readyState === 'complete' && self.crossOriginIsolated") != "true" and time.time() < deadline:
		time.sleep(0.5)


class MemoryWatch:
	"""the processes' own memory (anon: without the page cache) of this process's cgroup v2, on the dry run's server the
	scope that holds the browser too, after each example, so a tour that grows it names its biggest steps; silent where
	there is no cgroup v2 (macOS)"""
	def __init__(self):
		try:
			with open("/proc/self/cgroup") as entries:
				path = next(line.split("::", 1)[1].strip() for line in entries if line.startswith("0::"))
			self.folder = f"/sys/fs/cgroup{path}"
			self.last = self.anon()
		except (OSError, StopIteration, ValueError):
			self.folder = None
		self.growths = []
		self.at_peak = []
	def read(self, name):
		with open(os.path.join(self.folder, name)) as value:
			return value.read()
	def anon(self):
		return next(int(line.split()[1]) for line in self.read("memory.stat").splitlines() if line.startswith("anon "))
	def after(self, name):
		if self.folder:
			now = self.anon()
			if now > max((then for _, then, _ in self.growths), default=0):
				self.at_peak = self.processes()
			self.growths.append((now - self.last, now, name))
			self.last = now
	def report(self):
		if not self.folder:
			return
		try:
			peak = f", page cache included {int(self.read('memory.peak')) >> 20} MB"
		except OSError:
			peak = ""  # kernels before 5.19 keep no peak
		highest = max((now for _, now, _ in self.growths), default=self.last)
		largest = sorted(self.growths, reverse=True)[:MEMORY_GROWTHS_SHOWN]
		print(f"\nmemory: peak {highest >> 20} MB{peak}, at the end {self.last >> 20} MB; grew most: " + ", ".join(f"{name} +{growth >> 20} MB (to {now >> 20})" for growth, now, name in largest))
		print("memory at the peak, by process: " + ", ".join(f"{kind} {resident >> 10} MB" for resident, kind in self.at_peak[:MEMORY_GROWTHS_SHOWN]))
	def processes(self):
		"""the cgroup's processes, largest resident first: (KB, kind), a Chrome process by its --type (renderer, gpu-process …)"""
		found = []
		for pid in self.read("cgroup.procs").split():
			try:
				with open(f"/proc/{pid}/status") as status:
					resident = next((int(line.split()[1]) for line in status if line.startswith("VmRSS:")), 0)
				with open(f"/proc/{pid}/cmdline") as command:
					words = command.read().replace("\0", " ").split()  # Chrome rewrites its arguments as one
			except OSError:
				continue  # ended meanwhile
			kind = next((word.split("=", 1)[1] for word in words if word.startswith("--type=")), os.path.basename(words[0]))
			found.append((resident, kind))
		return sorted(found, reverse=True)


def guide_snippets(path=os.path.join(REPOSITORY, GUIDE_FILE)):
	"""(line, code, value, printed) of each guide fence that shows a value (```warp => value) or what it prints (a
	```printed fence right after it), as guide.js renders them"""
	with open(path, encoding="utf-8") as guide:
		lines = guide.read().split("\n")
	def fence_body(index):
		body = []
		index += 1
		while index < len(lines) and lines[index] != "```":
			body.append(lines[index])
			index += 1
		return "\n".join(body), index
	snippets, index = [], 0
	while index < len(lines):
		fence = SNIPPET_FENCE.match(lines[index])
		if fence:
			line = index + 1
			code, index = fence_body(index)
			printed = None
			if index + 1 < len(lines) and lines[index + 1] == PRINTED_FENCE:
				printed, index = fence_body(index + 1)
			if fence.group(1) is not None or printed is not None:
				snippets.append((line, code, fence.group(1), printed))
		index += 1
	return snippets


def snippet_wrong(code, value, printed):
	"""what the page shows otherwise than the guide says of a snippet: its value and printed lines"""
	answer = browser("eval", RUN_SNIPPET % (json.dumps(code), SAMPLE_ANSWER_SECONDS * 1000))
	shown = json.loads(json.loads(answer)) if answer.startswith('"') else {"value": f"(page gave no answer: {answer or browser_complaint})", "printed": ""}
	if shown.get("stuck"):
		return [f"no answer after {SAMPLE_ANSWER_SECONDS} s"]
	wrong = [f"value: {shown['value']!r}, the guide shows {value!r}"] if value is not None and shown["value"] != value else []
	if printed is not None and shown["printed"].rstrip("\n") != printed:
		wrong.append(f"printed: {shown['printed']!r}, the guide shows {printed!r}")
	return wrong


def check_examples(names, page_url=None, site=None):
	"""every example of the tour shows its value and prints its text in the playground (the local build, the collected
	`site` directory, or the deployed one at `page_url`), and neither loading the page nor showing an example or a sample
	writes an error or warning to the console (card console-errors); exit code 101 on a difference"""
	server = None
	serves_repository = not site and not page_url  # the checks that build files into data/ see them served
	if site:
		server = serve(os.path.abspath(site))
		page_url = f"http://127.0.0.1:{PORT}/"
	elif not page_url:
		if not os.path.isfile(os.path.join(REPOSITORY, "web", "playground", "warp-lib", "warp.wasm")):
			sys.exit("error: web/playground/warp-lib/warp.wasm is missing; build it with web/playground/build.sh")
		server = serve()
		page_url = f"http://127.0.0.1:{PORT}/web/playground/"
	open_page("about:blank")
	console = WebDriverConsole() if webdriver else ConsoleWatch()  # before the page, so its loading is watched too
	page_url = f"{page_url}{'&' if '?' in page_url else '?'}slow_start={SLOW_START_MS}"
	open_page(page_url)
	wait_for_isolation()
	wait_for_first_run()
	examples = json.loads(json.loads(browser("eval", "JSON.stringify(EXAMPLES)")))
	samples = json.loads(json.loads(browser("eval", "JSON.stringify(Object.keys(SAMPLES).sort())")))
	failures = []
	memory = MemoryWatch()
	def verdict(name, wrong):
		memory.after(name)
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
		if sample and isinstance(webdriver, FirefoxDriver) and name in FIREFOX_SKIPPED_SAMPLES:
			print(f"skip samples/{name} in Firefox: {FIREFOX_SKIPPED_SAMPLES[name]}")
			continue
		if sample and name in MEMORY_HUNGRY_SAMPLES:
			print(f"skip samples/{name}, memory hungry: {MEMORY_HUNGRY_SAMPLES[name]}")
			continue
		if sample:
			verdict(f"samples/{name}", run_sample(name))
		else:
			expected, shown = examples[name], show_example(name)
			if NO_GPU in str(shown.get("value")):
				print(f"skip {name}: this browser {NO_GPU}")
				shown = {**shown, "value": expected.get("value"), "canvases": expected.get("canvases")}
			verdict(name, [f"{part}: {shown.get(part)!r}, expected {expected[part]!r}" for part in ("value", "printed", "canvases", "clicked", "clickedPrinted", "kept", "keyed", "animated", "address") if part in expected and shown.get(part) != expected[part]] + [f"status: {shown.get('failed')}"] * bool(shown.get("failed")))
	snippets = guide_snippets() if GUIDE in names else []
	for line, code, value, printed in snippets:
		verdict(f"{GUIDE_FILE}:{line}", snippet_wrong(code, value, printed))
	extra_checks = [(check, wrong) for check, wrong in ((COMPLETION_KEYS, completion_keys_wrong), (STOP_BUTTON, stop_button_wrong), (CLEAR_BUTTON, clear_button_wrong), (MEMBER_COMPLETION, member_completion_wrong), (CLIPBOARD_READ, clipboard_read_wrong), (PAGE_ELEMENTS, page_elements_wrong), (GPU_PAGE, gpu_page_wrong), (VIEW_STEADY, view_steady_wrong), (GPU_JOBS, gpu_jobs_wrong), (SHADER_ERROR, shader_error_wrong), (MANY_TASKS, many_tasks_wrong), (FIX_BUTTON, fix_button_wrong), (PREVIOUS_FRAME, previous_frame_wrong), (STALE_VALUE, stale_value_wrong), (STOP_WHILE_LOADING, lambda: stop_while_loading_wrong(page_url)), (TEXT_HOLE, text_hole_wrong), (COMPONENT_HOLE, lambda: text_hole_wrong(COMPONENT_HOLE_CODE, "div", COMPONENT_HOLE_SHOWN)), (FILE_KEPT, lambda: file_kept_wrong(page_url)), (BRUSH, brush_wrong), (FINGERS, fingers_wrong), (FRAME_SHADERS, frame_shaders_wrong), (BUG_BUTTON, lambda: bug_button_wrong(page_url)), (REPORT, lambda: report_wrong(page_url)), *([(BUILT_SITE, built_site_wrong)] if serves_repository else [])) if not names or check in names]
	for check, wrong in extra_checks:
		verdict(check, wrong())
	console.stop()
	browser("close")
	if server:
		server.shutdown()
		if failures:
			show_served()
	memory.report()
	checked = len(chosen) + 1 + len(snippets) + len(extra_checks)
	print(f"\nexamples and samples: {checked - len(failures)} of {checked} show what they promise, without console errors" + (f"; failed: {', '.join(failures)}" if failures else ""))
	sys.exit(101 if failures else 0)


# one sample started in the page as its own file (%s: path and source as JSON), its report kept as JSON in
# window.sampleReport, once the fetches it started are answered (playground.js settled); asked for in short evals, since a long one loses agent-browser's channel ("CDP response channel
# closed" for gpu_visualizer under load)
# only the fields the verdict reads: a sample's whole report holds its pictures too, and a full-HD frame (samples/gpu/
# ringed_planet.warp) as JSON took this process past the dry run's memory cap
REPORT_FIELDS = ["error", "errors", "message", "value", "html", "printed", "printedWhole", "text", "stream", "paintings", "width", "height"]
# a sample's timers run this long before its output is taken, as scripts/samples.sh's WARP_EVENTS_FOR has them run
# natively (milliseconds)
SAMPLE_EVENTS_MS = 3500
START_SAMPLE = """window.sampleReport = undefined; playground.evaluate(%s, %s, { eventsFor: """ + str(SAMPLE_EVENTS_MS) + """ }).then(playground.settled).then(report => window.sampleReport = JSON.stringify(report, """ + json.dumps(REPORT_FIELDS) + """),
	failure => window.sampleReport = JSON.stringify({ error: true, value: String(failure) })); "started";"""


def page_output(report):
	"""what the page shows as the native warp prints it: the program's stdout, then its value unless that is ø; a page of
	markup its HTML (driver.js pageHtml), as the native warp prints page_render's"""
	# printedWhole: every frame's output (playground.js shows only an animation's last frame, its `printed`)
	printed = "".join(part["text"] for part in report.get("printedWhole", report.get("printed", [])) if part.get("stream", 1) == 1)
	value = report.get("html") or report.get("value") or ""
	if value in ("ø", "") or report.get("error"):
		return printed
	return printed + value + "\n"  # as the native warp shows it, a text in its quotes


def sample_verdict(report, native_output):
	"""(status, detail) of a sample's page run: ok when it printed what the native run printed"""
	if report.get("stuck"):
		return "TIME", f"the page gave no answer in {SAMPLE_ANSWER_SECONDS} s"
	if report.get("error"):
		first = (report.get("errors") or [report.get("value")])[0]
		first = first.get("message", first) if isinstance(first, dict) else first
		return "FAIL", str(first)[:200].replace("\n", " ")
	shown = page_output(report)
	if native_output is None:
		return "ran", "the native run failed or did not run: nothing to compare (test/samples.txt, scripts/samples.sh)"
	if MEASURED_TIME.sub("…", shown).rstrip() == MEASURED_TIME.sub("…", native_output).rstrip():
		return "ok", ""
	return "DIFF", f"page: {shown[:SHOWN_DIFFERENCE]!r}\n     native: {native_output[:SHOWN_DIFFERENCE]!r}"


def native_error(err_file):
	"""the native run's error of an expected failure: the first line of its stderr but warnings, None without one"""
	if not os.path.isfile(err_file):
		return None
	lines = [line for line in open(err_file, encoding="utf-8", errors="replace").read().splitlines() if line.strip() and not line.startswith("warning:")]
	return lines[0][:200] if lines else None


def expected_failure_verdict(page_error, native):
	"""an expected failure (EXPECTED_FAILURE) gives what the native run gives when it fails with the native error: ok,
	else EXP, which does not pass (decision exp-counts)"""
	if native is not None and page_error.strip() == native.strip():
		return "ok", ""
	return "EXP", f"page: {page_error!r}\n     native: {native!r}"


def window_verdict(report):
	"""(status, detail) of a sample opening a C library's window (raylib, SDL), which natively has no output to compare
	(scripts/samples.sh WIN): drew when the page ran it and showed its frames (host-media.js windowShown)"""
	status, detail = sample_verdict(report, "")
	if status in ("FAIL", "TIME"):
		return status, detail
	paintings = report.get("paintings") or []
	if not paintings:
		return "FAIL", "it drew nothing in the page"
	return "drew", f"{paintings[-1].get('width')}×{paintings[-1].get('height')}"


def page_report(source, path, page_url, animation_seconds=None):
	"""the page's report of the sample; a page that gave none (its renderer crashed, now and then after many runs) is
	opened anew and asked once more, so only a sample that fails twice in a fresh page counts as stuck"""
	for attempt in range(2):
		report = sample_answer(source, path, animation_seconds)
		if "stuck" not in report:
			return report
		open_samples_page(page_url)
	return report


def sample_answer(source, path, animation_seconds=None):
	"""the report of the sample started in the page, {stuck} when none came in SAMPLE_ANSWER_SECONDS; an animation
	stopped after `animation_seconds`, when given"""
	started = browser("eval", START_SAMPLE % (json.dumps(source), json.dumps(path)))
	if started != '"started"' and wait_for_playground():  # the page was still (re)loading: its API came since
		started = browser("eval", START_SAMPLE % (json.dumps(source), json.dumps(path)))
	if started != '"started"':
		return {"error": True, "stuck": None, "value": f"(the page did not start it: {(started or browser_complaint)[:200]})"}
	started_at = time.time()
	deadline = started_at + SAMPLE_ANSWER_SECONDS
	while time.time() < deadline:
		time.sleep(SAMPLE_POLL_SECONDS)
		if animation_seconds is not None and time.time() - started_at > animation_seconds:
			browser("eval", "playground.stopAnimation()")
		answer = browser("eval", "window.sampleReport ?? null")
		if answer not in ("null", ""):
			try:
				return json.loads(json.loads(answer))
			except ValueError:
				return {"error": True, "stuck": None, "value": f"(page gave no answer: {answer[:200]})"}
	return {"error": True, "stuck": True, "value": f"the page gave no answer in {SAMPLE_ANSWER_SECONDS} s"}


def open_samples_page(page_url):
	"""the page opened, isolated and its playground API defined (after a crashed renderer the first eval met none)"""
	open_page(page_url)
	wait_for_isolation()
	if not wait_for_playground():
		print(f"the page defined no playground API in {LAUNCH_SECONDS} s", file=sys.stderr)


def wait_for_playground():
	"""whether the page's playground API is defined, waiting up to LAUNCH_SECONDS for it (a sample that crashed the
	renderer leaves a page that reloads, the sweep's next sample met no API: card playground-samples)"""
	deadline = time.time() + LAUNCH_SECONDS
	while time.time() < deadline:
		if browser("eval", "typeof playground") == '"object"':
			return True
		time.sleep(0.5)
	return False


def check_samples(paths, tree):
	"""every sample (or the given ones) in the page served from `tree`, its output against the native run's"""
	native = dict(line.split(None, 1)[::-1] for line in open(os.path.join(tree, NATIVE_RESULTS), encoding="utf-8").read().split("\n") if line.strip() and not line.startswith("#"))
	# a failure's line names its reason after the path (scripts/samples.sh): `FAIL samples/x.warp: <reason>`
	native = {path.split(":", 1)[0].strip(): status for path, status in native.items()}
	# a run of some samples leaves the tracked results alone
	written = not paths
	# a sample removed since the native run is not in the tree any more
	paths = paths or sorted(path for path in native if os.path.isfile(os.path.join(tree, path)))
	data = os.path.join(tree, SAMPLES_WEB_DATA)
	os.makedirs(os.path.join(data, "out"), exist_ok=True)
	server = serve(tree)
	page_url = f"http://127.0.0.1:{PORT}/web/playground/?run_seconds={SAMPLE_RUN_SECONDS}"
	open_samples_page(page_url)
	lines, details = [], []
	for path in paths:
		output_name = path.replace("/", "_") + ".txt"
		source = open(os.path.join(tree, path), encoding="utf-8").read()
		if sample_host(source) in ("native", "none"):
			continue
		window = native.get(path) == "WIN"
		report = page_report(source, path, page_url, WINDOW_ANIMATION_SECONDS if window else None)
		if window:
			status, detail = window_verdict(report)
		else:
			native_file = os.path.join(tree, NATIVE_OUTPUTS, output_name)
			compared = native.get(path) == "ok" and os.path.isfile(native_file)
			native_output = open(native_file, encoding="utf-8", errors="replace").read() if compared else None
			status, detail = sample_verdict(report, native_output)
		wrong = checks_wrong(os.path.basename(path)[:-len(".warp")], source) if status in ("ok", "ran") and CHECK_LINE.search(source) else []
		if wrong:
			status, detail = "DIFF", "; ".join(wrong)
		if status == "FAIL" and EXPECTED_FAILURE.search(source):
			status, detail = expected_failure_verdict(detail, native_error(os.path.join(tree, NATIVE_OUTPUTS, path.replace("/", "_") + ".err")))
		elif status == "FAIL" and NATIVE_ONLY.search(source):
			status = "EXP"
		with open(os.path.join(data, "out", output_name), "w", encoding="utf-8") as shown:
			shown.write(page_output(report) if not report.get("error") else str(report.get("value")))
		print(f"{status:<4} {path}" + (f": {detail}" if detail and status != "ok" else ""))
		lines.append(f"{status:<4} {path}")
		details.append(f"{status:<4} {path}" + (f"\n     {detail}" if detail else ""))
	browser("close")
	server.shutdown()
	if written:
		open(os.path.join(tree, SAMPLES_WEB_RESULTS), "w", encoding="utf-8").write("\n".join(lines) + "\n")
	# a run of some samples keeps the whole run's report too
	open(os.path.join(data, "report.txt" if written else "report-some.txt"), "w", encoding="utf-8").write("\n".join(details) + "\n")
	counts = {status: sum(line.startswith(status) for line in lines) for status in ("ok", "drew", "DIFF", "FAIL", "EXP", "TIME", "ran", "WIN")}
	print(f"samples in the page: {counts['ok']} of {len(lines)} print what the native warp prints; " + ", ".join(f"{count} {status}" for status, count in counts.items() if count and status != "ok") + (f" -> {os.path.join(tree, SAMPLES_WEB_RESULTS)}" if written else ""))


# the page runs the cases [[code, file]…] one after the other, each as `warp test` runs one, its report pushed as it ends
RUN_CASES = """window.caseReports = []; (async cases => { for (const [code, file] of cases) {
	if (typeof playground !== "object") { window.caseReports.push({ error: true, value: "%s" }); return; }
	let report; try { report = await playground.evaluate(code, file, { asData: true }); } catch (failure) { report = { error: true, value: String(failure) }; }
	window.caseReports.push(report); } })(%s); "started";"""
TAKE_REPORTS = "JSON.stringify((window.caseReports ?? []).splice(0))"


def compiler_said(tree, *words):
	"""what the page's compiler printed for a command of extensions/commands/tests.warp, run natively in `tree`"""
	ran = subprocess.run(["warp", os.path.join(tree, PAGE_COMPILER), *words], cwd=tree, capture_output=True, text=True)
	if ran.returncode not in (0, 1):
		sys.exit(f"error: warp {PAGE_COMPILER} {words[0]} failed ({ran.returncode}): {ran.stderr[-500:]}")
	return ran.stdout


def case_written(work, run, report, milliseconds):
	"""the page's report of a run written as the native runner writes one (core/native/tests.warp run_ended)"""
	stdout = "".join(part["text"] for part in report.get("printed", []) if part.get("stream", 1) == 1)
	stderr = "".join(part["text"] for part in report.get("printed", []) if part.get("stream", 1) == 2)
	value = str(report.get("value") or "")
	status = "timeout" if report.get("stuck") or value.startswith(STOPPED) else "1" if report.get("error") else "0"
	files = {"out": stdout, "err": stderr + (value + "\n" if report.get("error") else ""), "status": status, "ms": str(int(milliseconds))}
	for extension, content in files.items():
		open(os.path.join(work, f"{run}.{extension}"), "w", encoding="utf-8", newline="").write(content)


def cases_ran(work, runs, page_url):
	"""each run of the work folder run in the page, CASE_BATCH at a time; a page that ends no run in CASE_ANSWER_SECONDS
	is opened anew, the run it stuck on written as timed out"""
	done, lost_at = 0, None
	while done < len(runs):
		batch = runs[done:done + CASE_BATCH]
		cases = [[open(os.path.join(work, f"{run}.code"), encoding="utf-8", newline="").read(), f"{folder}/{CASE_FILE}"] for run, folder in batch]
		if browser("eval", RUN_CASES % (PAGE_LOST, json.dumps(cases))) != '"started"':
			page_opened(page_url)
			continue
		ended, last = 0, time.time()
		while ended < len(batch):
			time.sleep(SAMPLE_POLL_SECONDS)
			answer = browser("eval", TAKE_REPORTS)
			reports = json.loads(json.loads(answer)) if answer.startswith('"') else []
			now = time.time()
			lost = any(PAGE_LOST in str(report.get("value")) for report in reports)
			for report in reports:
				if PAGE_LOST in str(report.get("value")):
					break
				case_written(work, batch[ended][0], report, (now - last) * 1000 / len(reports))
				ended += 1
			if lost:
				# a case that loses the page twice is that case's failure, so the run goes on past it
				current = batch[ended][0]
				if lost_at == current:
					case_written(work, current, {"error": True, "value": f"the page lost its API running it ({PAGE_LOST})"}, 0)
					ended += 1
					lost_at = None
				else:
					lost_at = current
				page_opened(page_url)
				break
			if reports:
				last = now
			elif now - last > CASE_ANSWER_SECONDS:
				case_written(work, batch[ended][0], {"stuck": True}, CASE_ANSWER_SECONDS * 1000)
				ended += 1
				page_opened(page_url)
				break
		done += ended
		print(f"     {done} of {len(runs)} runs")


def page_opened(page_url):
	"""the page opened anew with its playground API, else the run stops: verdicts of a page without one are no verdicts"""
	open_samples_page(page_url)
	if browser("eval", "typeof playground") != '"object"':
		sys.exit(f"error: the page has no playground API after it was opened anew ({page_url}): {browser_complaint}")


def verdicts_of(lines):
	"""case → ✓ or ✗ of verdict lines (test-verdicts --each, the stored baseline)"""
	return {found.group(2): found.group(1) for found in map(VERDICT.match, lines) if found}


@functools.cache
def native_only_lines(tree, path):
	"""the line numbers a `# native only:` line covers in the test file at `path`"""
	covered, inside = set(), False
	for number, line in enumerate(open(os.path.join(tree, path), encoding="utf-8"), 1):
		inside = bool(inside or NATIVE_ONLY.match(line)) and line.strip() != ""
		if inside:
			covered.add(number)
	return covered


def native_only(tree, case):
	"""whether a `# native only:` line of its test file covers the case"""
	path, line = CASE_PLACE.match(case).groups()
	return int(line) in native_only_lines(tree, path)


def check_tests(paths, tree):
	"""every case of the test files under `paths` (default test/) run in the page served from `tree`, its verdict by the
	page's compiler against the native one of the stored baseline: ok, PAGE (fails only in the page), BOTH (fails
	natively too), EXP (failing in the page where its test file says native only), NONE (failing, no native verdict stored)"""
	data = os.path.join(tree, TESTS_WEB_DATA)
	work = os.path.join(data, "work")
	subprocess.run(["rm", "-rf", work])
	os.makedirs(work)
	compiler_said(tree, "test-list", work, *paths)
	runs = [line.split("\t") for line in open(os.path.join(work, "runs"), encoding="utf-8").read().split("\n") if line]
	server = serve(tree)
	page_url = f"http://127.0.0.1:{PORT}/web/playground/?run_seconds={CASE_RUN_SECONDS}"
	open_samples_page(page_url)
	cases_ran(work, runs, page_url)
	browser("close")
	server.shutdown()
	said = compiler_said(tree, "test-verdicts", work, "--each", *paths)
	page = verdicts_of(said.split("\n"))
	if not page:
		sys.exit(f"error: test-verdicts gave no verdicts (the runs are kept in {work}): {said[-500:]}")
	baseline = subprocess.run([os.path.join(tree, "scripts", "baseline.sh"), *(paths or ["test"])], cwd=tree, capture_output=True, text=True)
	native = verdicts_of(baseline.stdout.split("\n"))
	status_of = lambda case: "ok" if page[case] == "✓" else "EXP" if native.get(case) == "✓" and native_only(tree, case) else {"✓": "PAGE", "✗": "BOTH"}.get(native.get(case), "NONE")
	lines = [f"{status_of(case):<4} {case}" for case in page]
	written = not paths  # a run of some folders leaves the tracked results alone
	if written:
		open(os.path.join(tree, TESTS_WEB_RESULTS), "w", encoding="utf-8").write("\n".join(lines) + "\n")
	open(os.path.join(data, "report.txt" if written else "report-some.txt"), "w", encoding="utf-8").write("\n".join(lines) + "\n")
	counts = {status: sum(line.startswith(status) for line in lines) for status in ("ok", "PAGE", "EXP", "BOTH", "NONE")}
	print(f"tests in the page: {counts['ok']} of {len(lines)} hold; " + ", ".join(f"{count} {status}" for status, count in counts.items() if count and status != "ok") + (f" -> {os.path.join(tree, TESTS_WEB_RESULTS)}" if written else ""))


WEBDRIVERS = {"--firefox": FirefoxDriver, "--safari": SafariDriver}


def chrome_allowed():
	"""Chrome runs only in CI (user, 2026-10-09: no test launches Chrome or Chromium on the Mac): --serve starts none,
	--firefox runs Firefox, --safari Safari; --samples runs headless Chrome (user 2026-11-01, notes/playground_plan.md),
	and so does --tests, its twin for test/ (night decision N6, notes/night_2026-11-02.md), and --examples, the tour and
	the guide, in headless Chromium, Chrome for Testing (user 2026-11-02)"""
	return bool(os.environ.get("CI")) or sys.argv[1:2] in (["--serve"], ["--samples"], ["--tests"], ["--examples"]) or any(name in sys.argv[2:] for name in WEBDRIVERS)


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
	if sys.argv[1:2] == ["--samples"]:
		return check_samples(sys.argv[2:], REPOSITORY)
	if sys.argv[1:2] == ["--tests"]:
		return check_tests(sys.argv[2:], REPOSITORY)
	if sys.argv[1:2] == ["--serve"]:
		serve()
		print(f"serving {REPOSITORY}\n  playground: http://127.0.0.1:{PORT}/web/playground/")
		threading.Event().wait()
	sys.exit(__doc__)


if __name__ == "__main__":
	main()
