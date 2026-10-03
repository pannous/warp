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
	browser("click", ".note button")
	time.sleep(1.5)
	after = (browser("get", "text", "#value"), "upto" in browser("get", "text", "#diagnostics"))
	silenced = before == ("6", True) and after == ("6", False)
	print(f"{'ok  ' if silenced else 'FAIL'} got it silences the upto warning: {before} → {after}")
	if not silenced: failures.append("got it")
	browser("eval", "playground.forgetAll()")

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
