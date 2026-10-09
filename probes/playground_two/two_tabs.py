"""card playground-two: two tabs of samples/todo_app.warp in one browser profile (one IndexedDB) each add a todo; the
table must keep both, with different ids. Serve the repository root on PORT after web/playground/build.sh debug:
    python3 -m http.server 18671 & python3 probes/playground_two/two_tabs.py
Prints what each tab shows (tab 2 keeps its page until it runs again); exits 1 when a todo is lost or two share an id."""
import json, os, pathlib, sys, time
from playwright.sync_api import sync_playwright

PORT = os.environ.get("PORT", "18671")
PAGE = f"http://localhost:{PORT}/web/playground/?debug"
SAMPLE = pathlib.Path(__file__).resolve().parents[2] / "samples/todo_app.warp"
SHOWN = '[...document.getElementById("rendered").shadowRoot.querySelectorAll("li")].map(li => li.textContent.trim())'
IDS = '[...document.getElementById("rendered").shadowRoot.querySelectorAll("li form")].map(form => form.getAttribute("action"))'


def run(page):
	# the page of an earlier run goes first, so the wait below sees this run's
	page.evaluate(f"""{{ const root = document.getElementById('rendered').shadowRoot; if (root) root.innerHTML = '';
		document.querySelector('.CodeMirror').CodeMirror.setValue({json.dumps(SAMPLE.read_text())}); document.getElementById('run').click() }}""")
	page.wait_for_function('document.getElementById("rendered").shadowRoot?.querySelector("form[action=\'/todos\']")', timeout=60000)


def add(page, title):
	before = len(page.evaluate(SHOWN))
	page.evaluate(f"""{{ const root = document.getElementById('rendered').shadowRoot;
		root.querySelector('input[name=title]').value = {json.dumps(title)};
		root.querySelector('form[action="/todos"] button').click() }}""")
	page.wait_for_function(f"{SHOWN}.length > {before}", timeout=30000)


def open_tab(context):
	page = context.new_page()
	page.goto(PAGE)
	page.wait_for_selector(".CodeMirror", timeout=60000)
	return page


with sync_playwright() as playwright:
	browser = playwright.chromium.launch()
	context = browser.new_context()
	first = open_tab(context)
	run(first)
	second = open_tab(context)
	run(second)
	add(first, "jam")
	time.sleep(0.5)  # a person's pace: the other tab hears of a row within ms (BroadcastChannel); at once it may not yet
	add(second, "tea")
	print("tab 2:", second.evaluate(SHOWN), second.evaluate(IDS))
	run(first)
	shown, ids = first.evaluate(SHOWN), first.evaluate(IDS)
	print("tab 1 after a new run:", shown, ids)
	browser.close()
	sys.exit(0 if shown == ["☐jam", "☐tea"] and len(set(ids)) == 2 else 1)
