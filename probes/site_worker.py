#!/usr/bin/env python3
"""Card site-worker: a built site whose main awaits tasks runs them on the task Workers (probes/site/parallel.wasp: three
one-second naps together, about 1 s, not 3 s one after the other on the page's thread). Builds the site with the warp
binary given (default: warp on PATH) into scratch/site_worker/, serves the repository like test_in_browser.py (with
cross-origin isolation), opens the page and reads what it shows once the hydration ran main again, and after a click on its button.
Card site-worker-js: the Worker gets the tab's session values (one set before a reload) and a clipboard write reaches
the page (navigator.clipboard.writeText patched: headless Chrome has no clipboard).
Card site-worker-step: a site with routes (probes/site/routed_tasks.wasp) runs in the Worker too; a link and the back
button reach it through the page, which shows the route and focuses its heading.
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

ROUTED_PROGRAM = os.path.join(REPOSITORY, "probes", "site", "routed_tasks.wasp")
ROUTE_SHOWN_SECONDS = 2
SHOWN_HEADING = "document.querySelector('#wasp-root h1')?.textContent + ' ' + document.activeElement.tagName"

def build(warp, program):
	"""the site of `program` built in FOLDER, its directory"""
	os.makedirs(FOLDER, exist_ok=True)
	shutil.copy(program, FOLDER)
	name = os.path.splitext(os.path.basename(program))[0]
	subprocess.run([warp, "build", "--site", os.path.join(FOLDER, name + ".wasp")], check=True, capture_output=True, text=True)
	return os.path.join(FOLDER, name + "-site")

def check_routes(warp):
	"""the routed site served at its root (routes read location.pathname): home, a link, the back button"""
	page.PORT = 0
	page.serve(None, build(warp, ROUTED_PROGRAM))
	page.open_page(f"http://127.0.0.1:{page.PORT}/")
	page.wait_for_isolation()
	time.sleep(ROUTE_SHOWN_SECONDS)
	shown = [page.browser("eval", SHOWN_HEADING)]
	page.browser("eval", "document.querySelector('#wasp-root a').click()")
	time.sleep(ROUTE_SHOWN_SECONDS)
	shown.append(page.browser("eval", SHOWN_HEADING))
	page.browser("eval", "history.back()")
	time.sleep(ROUTE_SHOWN_SECONDS)
	shown.append(page.browser("eval", SHOWN_HEADING + " + ' ' + location.pathname"))
	expected = ['"Home 1 BODY"', '"Other H1"', '"Home 1 H1 /"']
	ok = shown == expected
	print(f"{'ok' if ok else 'FAIL'}: routes in the Worker show {shown}" + ("" if ok else f", expected {expected}"))
	return ok

def main():
	warp = sys.argv[1] if len(sys.argv) > 1 else "warp"
	shutil.rmtree(FOLDER, ignore_errors=True)
	path = "/" + os.path.relpath(build(warp, PROGRAM), REPOSITORY) + "/index.html"
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
	ok = all(expected in shown for expected in (EXPECTED, CLICKED, SESSION_SHOWN)) and COPIED in copied
	print(f"{'ok' if ok else 'FAIL'}: the page shows {shown}, copied {copied}" + ("" if ok else f", expected {EXPECTED!r}, {CLICKED!r}, {SESSION_SHOWN!r} and copied {COPIED!r}"))
	routes_ok = check_routes(warp)
	page.browser("close")
	sys.exit(0 if ok and routes_ok else 1)

main()
