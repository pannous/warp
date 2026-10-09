#!/usr/bin/env python3
"""Headless probe of web/playground: serves the repository, loads the page with agent-browser (headless Chrome),
evaluates the tour, the Ask flow and every samples/*.warp of the playground's menu in the browser, and compares each
value with the CLI's (`warp --no-ask`); exits 1 on any failure or difference. Needs web/playground/build.sh run first
(`optimized`; `debug` too for the debug-build check) and builds the native warp binary itself.
Usage: probes/web_playground.py [sample names…]"""
import json, os, re, shutil, subprocess, sys, time, http.server, threading, functools

PORT = 8732
SESSION = "web-playground-probe"
REPOSITORY = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PAGE = f"http://127.0.0.1:{PORT}/web/playground/"
CLI_TIMEOUT = 20
RUN_TIMEOUT = 30  # seconds a page run may take (the first `use python` loads Pyodide)
# expected values of the page itself: code → value
CHECKS = {
	"3+3": "6",
	"fib := it < 2 ? it : fib(it - 1) + fib(it - 2)\nfib(10)": "55",
	'print "hi"; 7': "7",
	"[1 2.5 'x' (a:3)]": "[1 2.5 'x' a:3]",
	"xs=[1 2]; xs#5": 'Error("index out of range: 5 not in 1…2")',
	"2^100": "1267650600228229401496703205376",
	"1/3": "1/3",
	# packages and module files come through the page (warp_host.fetch, host.read of a URL)
	'use uniscript; uniscript("<:alpha>")': '"α"',
	"include tests/fixtures/counter; counter": "11",
	# `use python` loads Pyodide from the CDN in the worker (worker.js loadPython, host.js pythonCall)
	"use python math; math.floor(2.5) + 1": "3",
	"use python math; math.factorial(25)": "15511210043330985984000000",
	# paint draws on a canvas of the page (host.js paint, playground.js showPaintings); the value is the last line
	"paint([0, 1, 1, 0], 2, 2); 7": "7",
}
DEBUG_BUILD = os.path.join(REPOSITORY, "web", "playground", "warp.debug.wasm")
EXCLUDED = os.path.join(REPOSITORY, "web", "playground", "excluded_samples.txt")
TEST_LINE = r"^\s*test\b"  # a program with tests runs them in the page, as `warp test` does (lowering/test_blocks.rs)
UPTO = "x=0; for i in 1 upto 4 {x+=i}; x"


def browser(*arguments):
	result = subprocess.run(["agent-browser", "--session", SESSION, *arguments], capture_output=True, text=True, timeout=600)
	return result.stdout.strip()


def evaluate_in_page(codes):
	"""values of the codes, evaluated one after another by the page's worker"""
	script = f"""(async () => {{ const reports = []; for (const code of {json.dumps(codes)})
		reports.push(await playground.evaluate(code)); return JSON.stringify(reports); }})()"""
	return json.loads(json.loads(browser("eval", script)))


def settled(*command):
	"""the browser command, then the page's run it starts finished: #status shows its time (or a failure), so a slow run
	(the worker still busy with Pyodide) is never read half done, as fixed sleeps once did"""
	browser("eval", "document.getElementById('status').textContent = ''")
	browser(*command)
	deadline = time.time() + RUN_TIMEOUT
	while browser("get", "text", "#status") in ("", "running…") and time.time() < deadline:
		time.sleep(0.2)


def shown_value(report):
	"""the page's value as the CLI prints it: markup the page renders as DOM (report.html) the CLI prints as its HTML
	text, the page's » line shows the markup node; that is presentation, the program's value is the same"""
	return report["html"] if report.get("html") and not report.get("error") else report["value"]


def build_cli():
	"""this checkout's warp, copied away at once: copies of warp share the target dir's debug/warp"""
	build = subprocess.run(["cargo", "build", "--offline", "--bin", "warp", "--message-format", "json"], capture_output=True, text=True, cwd=REPOSITORY)
	built = [json.loads(line).get("executable") for line in build.stdout.splitlines() if '"executable"' in line]
	copy = os.path.join(REPOSITORY, "scratch", "web_playground_warp")
	os.makedirs(os.path.dirname(copy), exist_ok=True)
	shutil.copy(next(path for path in built if path), copy)
	return copy


def cli_value(warp, name, code):
	"""the CLI's value of a sample as the page shows it: the page's Run runs a program's `test` lines as `warp test`
	does (pipeline::for_tests), its summary a text; a program whose value is ø shows no » line, only what it printed"""
	tests = re.search(TEST_LINE, code, re.MULTILINE)
	command = ["test", os.path.join("samples", name + ".warp")] if tests else ["eval", code]
	try:
		run = subprocess.run([warp, "--no-ask", *command], capture_output=True, text=True, timeout=CLI_TIMEOUT, cwd=REPOSITORY)
	except subprocess.TimeoutExpired:
		return "(timeout)"
	if tests:
		summary = run.stdout.strip().rsplit("\n", 1)[-1]
		return json.dumps(summary, ensure_ascii=False) if run.returncode == 0 else f"Error({json.dumps(summary, ensure_ascii=False)})"
	return run.stdout.rsplit("» ", 1)[-1].strip() if "» " in run.stdout else "ø"


def page_samples():
	"""samples/*.warp the playground's menu offers: excluded_samples.txt names the others (a window, a server), which
	the probe never runs, so no native window opens"""
	excluded = {line.split("\t")[0] for line in open(EXCLUDED, encoding="utf-8") if line.strip() and not line.startswith("#")}
	return sorted(name[:-5] for name in os.listdir(os.path.join(REPOSITORY, "samples")) if name.endswith(".warp") and name[:-5] not in excluded)


def serve():
	class Quiet(http.server.SimpleHTTPRequestHandler):
		def log_message(self, *_):
			pass
	handler = functools.partial(Quiet, directory=REPOSITORY)
	server = http.server.ThreadingHTTPServer(("127.0.0.1", PORT), handler)
	threading.Thread(target=server.serve_forever, daemon=True).start()
	return server


def check_debug_build(failures):
	browser("open", PAGE + "?debug")
	for _ in range(60):
		if browser("get", "text", "#status") not in ("", "loading the compiler…"):
			break
		time.sleep(1)
	debug = evaluate_in_page(["3+3"])[0]["value"]
	ok = debug == "6" and "debug build" in browser("get", "text", "#build")
	print(f"{'ok  ' if ok else 'FAIL'} the debug build evaluates 3+3 → {debug}")
	if not ok: failures.append("debug build")
	browser("open", PAGE)


def main():
	failures = []
	warp = build_cli()
	server = serve()
	browser("open", PAGE)
	# runs asked for while the compiler still loads queue up (they once raced: "pending is undefined" in the run timer)
	early = browser("eval", "Promise.all([playground.evaluate('1+1'), playground.evaluate('2+2')]).then(reports => reports.map(r => r.value).join(' '))")
	ok = early == '"2 4"'
	print(f"{'ok  ' if ok else 'FAIL'} runs asked for while the compiler loads: {early}")
	if not ok: failures.append("early runs")
	for _ in range(30):
		if browser("get", "text", "#status") not in ("", "loading the compiler…"):
			break
		time.sleep(0.5)
	print("status:", browser("get", "text", "#status"))

	codes = list(CHECKS)
	for (code, expected), report in zip(CHECKS.items(), evaluate_in_page(codes)):
		ok = report["value"] == expected
		print(f"{'ok  ' if ok else 'FAIL'} {code!r} → {report['value']}")
		if not ok: failures.append(code)

	# an ambiguity is a warning; "got it" silences its topic and never changes the value
	browser("eval", "playground.forgetAll()")
	settled("select", "#examples", "ambiguity")
	before = (browser("get", "text", "#value"), "upto" in browser("get", "text", "#diagnostics"))
	settled("click", ".got-it")
	after = (browser("get", "text", "#value"), "upto" in browser("get", "text", "#diagnostics"))
	silenced = before == ("6", True) and after == ("6", False) and "upto@" in browser("get", "text", "#acknowledged")
	print(f"{'ok  ' if silenced else 'FAIL'} got it silences this upto expression: {before} → {after}")
	if not silenced: failures.append("got it")
	browser("eval", "playground.forgetAll()")

	# "// got it" says it in the code: the comment on the warning's line silences it
	settled("select", "#examples", "ambiguity")
	settled("eval", "[...document.querySelectorAll('.got-it')].find(button => button.textContent === '// got it').click()")
	commented = ("// got it" in json.loads(browser("eval", "playground.code()")), "upto" in browser("get", "text", "#diagnostics"), browser("get", "text", "#value"))
	ok = commented == (True, False, "6")
	print(f"{'ok  ' if ok else 'FAIL'} the // got it button comments the line and silences it: {commented}")
	if not ok: failures.append("got it comment")

	# "I meant: ..." rewrites the code at the warning and runs it again: the inclusive reading of `1 upto 4` is 1+2+3+4
	settled("select", "#examples", "ambiguity")
	settled("eval", "[...document.querySelectorAll('.apply-fix')].find(button => button.textContent === 'I meant: ...').click()")
	fixed = (browser("get", "text", "#value"), "for i in 1 ... 4" in json.loads(browser("eval", "playground.code()")), "upto" in browser("get", "text", "#diagnostics"))
	ok = fixed == ("10", True, False)
	print(f"{'ok  ' if ok else 'FAIL'} the fix button rewrites `upto` to `...` and runs again: {fixed}")
	if not ok: failures.append("fix button")
	# an ambiguity error offers its readings too
	settled("eval", "playground.setCode('square:=it*it; square 3 + square 4')")
	settled("eval", "[...document.querySelectorAll('.apply-fix')].find(button => button.textContent === 'I meant: square(3) + square(4)').click()")
	value = browser("get", "text", "#value")
	print(f"{'ok  ' if value == '25' else 'FAIL'} the fix of an ambiguous braceless call: {value}")
	if value != "25": failures.append("error fix button")
	# a fix of several edits: the kebab key renamed at the key and at its read (P81)
	settled("eval", "playground.setCode('a=5; b=1; a-b:2; a-b')")
	settled("eval", "[...document.querySelectorAll('.apply-fix')].find(button => button.textContent === 'I meant: a_b').click()")
	renamed = (browser("get", "text", "#value"), json.loads(browser("eval", "playground.code()")))
	ok = renamed == ("2", "a=5; b=1; a_b:2; a_b")
	print(f"{'ok  ' if ok else 'FAIL'} a fix of several edits renames the key and its read: {renamed}")
	if not ok: failures.append("multi-edit fix button")

	# page events (notes/signals.md phase 7): a click on the result runs `on click`, the last line (total) is shown anew
	settled("select", "#examples", "events")
	for _ in range(2):
		browser("click", "#value")
		time.sleep(0.7)
	clicked = (browser("get", "text", "#value"), browser("get", "text", "#printed").count("click at"))
	ok = clicked == ("6", 2)
	print(f"{'ok  ' if ok else 'FAIL'} two clicks run on click and show total again: {clicked}")
	if not ok: failures.append("page events")

	# the debug build (?debug, warp.debug.wasm) compiles the same programs; `build.sh optimized` alone leaves it out
	if os.path.exists(DEBUG_BUILD):
		check_debug_build(failures)
	else:
		print(f"skip the debug build: no {os.path.relpath(DEBUG_BUILD, REPOSITORY)} (web/playground/build.sh debug)")

	names = sys.argv[1:] or page_samples()
	sources = [open(os.path.join(REPOSITORY, "samples", name + ".warp"), encoding="utf-8").read() for name in names]
	reports = evaluate_in_page(sources)
	same = 0
	for name, source, report in zip(names, sources, reports):
		cli = cli_value(warp, name, source)
		if shown_value(report) == cli:
			same += 1
		else:
			failures.append(f"samples/{name}")
			print(f"DIFF samples/{name}.warp\n     page: {shown_value(report)[:200]}\n     cli:  {cli[:200]}")
	print(f"samples: {same}/{len(names)} give the CLI's value in the page")
	browser("close")
	server.shutdown()
	sys.exit(1 if failures else 0)


if __name__ == "__main__":
	main()
