#!/usr/bin/env python3
"""Headless probe of web/playground: serves the repository, loads the page with agent-browser (headless Chrome),
evaluates the tour, the Ask flow and every samples/*.wasp in the browser, and compares each value with the CLI's
(`warp --no-ask`). Needs web/playground/build.sh run first and the native warp binary built.
Usage: probes/web_playground.py [sample names…]"""
import json, os, shutil, subprocess, sys, time, http.server, threading, functools

PORT = 8732
SESSION = "web-playground-probe"
REPOSITORY = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PAGE = f"http://127.0.0.1:{PORT}/web/playground/"
CLI_TIMEOUT = 20
# expected values of the page itself: code → value
CHECKS = {
	"3+3": "6",
	"fib := it < 2 ? it : fib(it - 1) + fib(it - 2)\nfib(10)": "55",
	'print "hi"; 7': "7",
	"[1 2.5 'x' (a:3)]": "[1 2.5 'x' a:3]",
	"xs=[1 2]; xs#5": 'Error("index out of range")',
	"2^100": "1267650600228229401496703205376",
	"1/3": "1/3",
	# packages and module files come through the page (warp_host.fetch, host.read of a URL)
	'use uniscript; uniscript("<:alpha>")': '"α"',
	"include tests/fixtures/counter; counter": "11",
}
UPTO = "x=0; for i in 1 upto 4 {x+=i}; x"


def browser(*arguments):
	result = subprocess.run(["agent-browser", "--session", SESSION, *arguments], capture_output=True, text=True, timeout=600)
	return result.stdout.strip()


def evaluate_in_page(codes):
	"""values of the codes, evaluated one after another by the page's worker"""
	script = f"""(async () => {{ const reports = []; for (const code of {json.dumps(codes)})
		reports.push(await playground.evaluate(code)); return JSON.stringify(reports); }})()"""
	return json.loads(json.loads(browser("eval", script)))


def build_cli():
	"""this checkout's warp, copied away at once: copies of warp share the target dir's debug/warp"""
	build = subprocess.run(["cargo", "build", "--offline", "--bin", "warp", "--message-format", "json"], capture_output=True, text=True, cwd=REPOSITORY)
	built = [json.loads(line).get("executable") for line in build.stdout.splitlines() if '"executable"' in line]
	copy = os.path.join(REPOSITORY, "scratch", "web_playground_warp")
	os.makedirs(os.path.dirname(copy), exist_ok=True)
	shutil.copy(next(path for path in built if path), copy)
	return copy


def cli_value(warp, code):
	try:
		run = subprocess.run([warp, "--no-ask", "eval", code], capture_output=True, text=True, timeout=CLI_TIMEOUT, cwd=REPOSITORY)
	except subprocess.TimeoutExpired:
		return "(timeout)"
	return run.stdout.rsplit("» ", 1)[-1].strip() if "» " in run.stdout else run.stdout.strip()


def serve():
	class Quiet(http.server.SimpleHTTPRequestHandler):
		def log_message(self, *_):
			pass
	handler = functools.partial(Quiet, directory=REPOSITORY)
	server = http.server.ThreadingHTTPServer(("127.0.0.1", PORT), handler)
	threading.Thread(target=server.serve_forever, daemon=True).start()
	return server


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
	browser("select", "#examples", "ambiguity")
	time.sleep(1.5)
	before = (browser("get", "text", "#value"), "upto" in browser("get", "text", "#diagnostics"))
	browser("click", ".got-it")
	time.sleep(1.5)
	after = (browser("get", "text", "#value"), "upto" in browser("get", "text", "#diagnostics"))
	silenced = before == ("6", True) and after == ("6", False) and "upto@" in browser("get", "text", "#acknowledged")
	print(f"{'ok  ' if silenced else 'FAIL'} got it silences this upto expression: {before} → {after}")
	if not silenced: failures.append("got it")
	browser("eval", "playground.forgetAll()")

	# "// got it" says it in the code: the comment on the warning's line silences it
	browser("select", "#examples", "ambiguity")
	time.sleep(1.5)
	browser("eval", "[...document.querySelectorAll('.got-it')].find(button => button.textContent === '// got it').click()")
	time.sleep(1.5)
	commented = ("// got it" in json.loads(browser("eval", "playground.code()")), "upto" in browser("get", "text", "#diagnostics"), browser("get", "text", "#value"))
	ok = commented == (True, False, "6")
	print(f"{'ok  ' if ok else 'FAIL'} the // got it button comments the line and silences it: {commented}")
	if not ok: failures.append("got it comment")

	# "I meant: ..." rewrites the code at the warning and runs it again: the inclusive reading of `1 upto 4` is 1+2+3+4
	browser("select", "#examples", "ambiguity")
	time.sleep(1.5)
	browser("eval", "[...document.querySelectorAll('.apply-fix')].find(button => button.textContent === 'I meant: ...').click()")
	time.sleep(1.5)
	fixed = (browser("get", "text", "#value"), "for i in 1 ... 4" in json.loads(browser("eval", "playground.code()")), "upto" in browser("get", "text", "#diagnostics"))
	ok = fixed == ("10", True, False)
	print(f"{'ok  ' if ok else 'FAIL'} the fix button rewrites `upto` to `...` and runs again: {fixed}")
	if not ok: failures.append("fix button")
	# an ambiguity error offers its readings too
	browser("eval", "playground.setCode('square:=it*it; square 3 + square 4')")
	time.sleep(1.5)
	browser("eval", "[...document.querySelectorAll('.apply-fix')].find(button => button.textContent === 'I meant: square(3) + square(4)').click()")
	time.sleep(1.5)
	value = browser("get", "text", "#value")
	print(f"{'ok  ' if value == '25' else 'FAIL'} the fix of an ambiguous braceless call: {value}")
	if value != "25": failures.append("error fix button")
	# a fix of several edits: the kebab key renamed at the key and at its read (P81)
	browser("eval", "playground.setCode('a=5; b=1; a-b:2; a-b')")
	time.sleep(1.5)
	browser("eval", "[...document.querySelectorAll('.apply-fix')].find(button => button.textContent === 'I meant: a_b').click()")
	time.sleep(1.5)
	renamed = (browser("get", "text", "#value"), json.loads(browser("eval", "playground.code()")))
	ok = renamed == ("2", "a=5; b=1; a_b:2; a_b")
	print(f"{'ok  ' if ok else 'FAIL'} a fix of several edits renames the key and its read: {renamed}")
	if not ok: failures.append("multi-edit fix button")

	# the debug build (?debug, warp.debug.wasm) compiles the same programs
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

	names = sys.argv[1:] or sorted(name[:-5] for name in os.listdir(os.path.join(REPOSITORY, "samples")) if name.endswith(".wasp"))
	sources = [open(os.path.join(REPOSITORY, "samples", name + ".wasp"), encoding="utf-8").read() for name in names]
	reports = evaluate_in_page(sources)
	same = 0
	for name, source, report in zip(names, sources, reports):
		cli = cli_value(warp, source)
		if report["value"] == cli:
			same += 1
		else:
			print(f"DIFF samples/{name}.wasp\n     page: {report['value'][:200]}\n     cli:  {cli[:200]}")
	print(f"samples: {same}/{len(names)} give the CLI's value in the page")
	browser("close")
	server.shutdown()
	sys.exit(1 if failures else 0)


if __name__ == "__main__":
	main()
