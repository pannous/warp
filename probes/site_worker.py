#!/usr/bin/env python3
"""Card site-worker: a built site whose main awaits tasks runs them on the task Workers (probes/site/parallel.wasp: three
one-second naps together, about 1 s, not 3 s one after the other on the page's thread). Builds the site with the warp
binary given (default: warp on PATH) into scratch/site_worker/, serves the repository like test_in_browser.py (with
cross-origin isolation), opens the page and reads what it shows once the hydration ran main again, and after a click on its button.
Card site-worker-js: the Worker gets the tab's session values (one set before a reload) and a clipboard write reaches
the page (navigator.clipboard.writeText patched: headless Chrome has no clipboard).
Usage: probes/site_worker.py [warp binary]"""
import os, shutil, subprocess, sys, time
REPOSITORY = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(REPOSITORY, "web", "playground"))
import test_in_browser as page

PROGRAM = os.path.join(REPOSITORY, "probes", "site", "parallel.wasp")
FOLDER = os.path.join(REPOSITORY, "scratch", "site_worker")
SHOWN_AFTER_SECONDS = 6  # main runs again in the page: 1 s on the task Workers, 3 s inline, plus loading
EXPECTED = "tasks ran together"
CLICKED = "clicked 1"  # the button's handler ran in the Worker and the page morphed in its markup
SESSION_SHOWN = "session says hi"
SESSION_ITEM = ("wasp session greeting", '"hi"')  # markup.js SESSION_PREFIX, the value as JSON
COPIED = "copied 1"
RECORD_CLIPBOARD = "navigator.clipboard.writeText = text => { window.copied = text; return Promise.resolve(); }"

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
	page.browser("eval", f"sessionStorage.setItem({SESSION_ITEM[0]!r}, {SESSION_ITEM[1]!r}); location.reload()")
	time.sleep(SHOWN_AFTER_SECONDS)
	page.browser("eval", RECORD_CLIPBOARD)
	page.browser("eval", "document.querySelector('#wasp-root button').click()")
	time.sleep(1)
	shown = page.browser("eval", "document.getElementById('wasp-root').textContent")
	copied = page.browser("eval", "window.copied")
	page.browser("close")
	ok = all(expected in shown for expected in (EXPECTED, CLICKED, SESSION_SHOWN)) and COPIED in copied
	print(f"{'ok' if ok else 'FAIL'}: the page shows {shown}, copied {copied}" + ("" if ok else f", expected {EXPECTED!r}, {CLICKED!r}, {SESSION_SHOWN!r} and copied {COPIED!r}"))
	sys.exit(0 if ok else 1)

main()
