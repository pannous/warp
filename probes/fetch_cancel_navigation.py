#!/usr/bin/env python3
"""Card fetch-cancel: a fetch still pending when the page goes to another path drops its reply (host-tasks.js
dropPendingFetches); a reply in before that stays. Serves the playground like test_in_browser.py (web/playground/build.sh
first), runs a routes program whose fetch replies after `delay` ms, follows a link after `before_click` ms, and reads
how many replies the handler saw.
Usage: probes/fetch_cancel_navigation.py"""
import json, os, sys
sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "web", "playground"))
import test_in_browser as page

REPLY_MILLISECONDS = 800
# (fetch delay, wait before the click, value shown at /other)
CASES = [(REPLY_MILLISECONDS, 0, 'p:"replies 0"'), (0, REPLY_MILLISECONDS, 'p:"replies 1"')]

def program(delay):
	return f'''replies = 0
users := fetch "/__stub__?delay={delay}&body=hi"
on change users {{ replies += 1 }}
route "/" {{ div{{ a{{ href: "/other" "other" }} }} }}
route "/other" {{ p{{ "replies " + replies }} }}'''

def shown_after_click(delay, before_click):
	script = f"""(async () => {{
		const pause = milliseconds => new Promise(done => setTimeout(done, milliseconds));
		playground.setCode({json.dumps(program(delay))});
		await pause(500); // run while typing shows it
		while (document.getElementById("status").textContent === "running…") await pause(50);
		await pause({before_click});
		document.getElementById("rendered").shadowRoot.querySelector("a").click();
		await pause({REPLY_MILLISECONDS * 2});
		return JSON.stringify([document.getElementById("value").textContent, document.getElementById("address").value]);
	}})()"""
	return json.loads(json.loads(page.browser("eval", script)))

def main():
	page.serve(None)
	page.open_page(f"http://127.0.0.1:{page.PORT}/web/playground/")
	page.wait_for_isolation()
	failures = 0
	for delay, before_click, expected in CASES:
		value, address = shown_after_click(delay, before_click)
		ok = value == expected and address == "/other"
		failures += not ok
		print(f"{'ok  ' if ok else 'FAIL'} reply after {delay} ms, click after {before_click} ms: {value!r} at {address!r}" + ("" if ok else f", expected {expected!r} at '/other'"))
	page.browser("close")
	sys.exit(1 if failures else 0)

main()
