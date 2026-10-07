#!/usr/bin/env python3
"""Card site-worker: a built site whose main awaits tasks runs them on the task Workers (probes/site/parallel.wasp: three
one-second naps together, about 1 s, not 3 s one after the other on the page's thread). Builds the site with the warp
binary given (default: warp on PATH) into scratch/site_worker/, serves the repository like test_in_browser.py (with
cross-origin isolation), opens the page and reads what it shows once the hydration ran main again.
Usage: probes/site_worker.py [warp binary]"""
import os, shutil, subprocess, sys, time
REPOSITORY = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(REPOSITORY, "web", "playground"))
import test_in_browser as page

PROGRAM = os.path.join(REPOSITORY, "probes", "site", "parallel.wasp")
FOLDER = os.path.join(REPOSITORY, "scratch", "site_worker")
SHOWN_AFTER_SECONDS = 6  # main runs again in the page: 1 s on the task Workers, 3 s inline, plus loading
EXPECTED = "tasks ran together"

def build(warp):
	shutil.rmtree(FOLDER, ignore_errors=True)
	os.makedirs(FOLDER)
	shutil.copy(PROGRAM, FOLDER)
	subprocess.run([warp, "build", "--site", os.path.join(FOLDER, "parallel.wasp")], check=True, capture_output=True, text=True)
	return "/scratch/site_worker/parallel-site/index.html"

def main():
	path = build(sys.argv[1] if len(sys.argv) > 1 else "warp")
	page.serve(None)
	page.open_page(f"http://127.0.0.1:{page.PORT}{path}")
	page.wait_for_isolation()
	time.sleep(SHOWN_AFTER_SECONDS)
	shown = page.browser("eval", "document.getElementById('wasp-root').textContent")
	page.browser("close")
	ok = EXPECTED in shown
	print(f"{'ok' if ok else 'FAIL'}: the page shows {shown}" + ("" if ok else f", expected {EXPECTED!r}"))
	sys.exit(0 if ok else 1)

main()
