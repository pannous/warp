# Card tour-firefox-stall: how often does the compiler's worker stall while starting in headless Firefox, and at which
# stage? Loads the playground of a collected site LOADS times, each in a fresh Firefox (the CI stalls all hit the tour's
# first page, right after Firefox started), and follows playground.state() until the worker is ready; a load still
# starting after STALL_SECONDS counts as a stall at its stage. Prints each load's seconds from open to ready.
# CI only (no browser on the Mac): python3 probes/firefox_start/starts.py _site [loads]
import collections, json, os, sys, time

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "..", "web", "playground"))
import test_in_browser as runner

LOADS = int(sys.argv[2]) if len(sys.argv) > 2 else 60
STALL_SECONDS = 25
POLL_SECONDS = 0.25
STATE = "JSON.stringify(typeof playground === 'object' ? playground.state() : null)"


def state():
	shown = runner.browser("eval", STATE)
	return json.loads(json.loads(shown)) if isinstance(shown, str) and shown.startswith('"') else None


def one_load(url):
	"""[(stage, seconds)] of this load, and whether its worker got ready"""
	began = time.time()
	runner.open_page(url)
	stages = [["(opening)", began]]
	while time.time() - began < STALL_SECONDS:
		current = state() or {"worker": "no playground yet", "ready": False}
		if not stages or stages[-1][0] != current["worker"]:
			stages.append([current["worker"], time.time()])
		if current["ready"]:
			return durations(stages + [["ready", time.time()]]), True
		time.sleep(POLL_SECONDS)
	return durations(stages + [["(gave up)", time.time()]]), False


def durations(stages):
	return [(stage, round(next_at - at, 2)) for (stage, at), (_, next_at) in zip(stages, stages[1:])]


def main():
	server = runner.serve(None, os.path.abspath(sys.argv[1]))
	stalls = collections.Counter()
	for load in range(1, LOADS + 1):
		runner.firefox = runner.FirefoxDriver()
		runner.open_page("about:blank")  # as the tour does
		stages, ready = one_load(f"http://127.0.0.1:{runner.PORT}/?slow_start={runner.SLOW_START_MS}")
		runner.firefox.command("close")
		runner.firefox.process.wait()
		total = sum(seconds for _, seconds in stages)
		print(f"{load:3} {'ready' if ready else 'STALL'} {total:5.1f} s  {stages}", flush=True)
		if not ready:
			stalls[stages[-1][0]] += 1
	print(f"\n{sum(stalls.values())} stalls in {LOADS} loads: {dict(stalls)}")
	server.shutdown()


main()
