#!/usr/bin/env python3
"""card g_oc54: pressing Run clears the last run's errors, underlines and printed text at once (headless Chrome, after
web/playground/build.sh)"""
import json, os, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "..", "web", "playground"))
import test_in_browser as page

FAILING = 'print "old"\nundefined_word(1)'
SLOW = "sleep 1500 ms\n1"
page.serve(None)
page.open_page(f"http://127.0.0.1:{page.PORT}/web/playground/")
time.sleep(3)
shown = lambda: page.browser("eval", """JSON.stringify({ diagnostics: document.getElementById("diagnostics").textContent,
	printed: document.getElementById("printed").textContent, marks: document.querySelectorAll(".marked-error").length,
	value: document.getElementById("value").textContent })""")
page.browser("eval", f"playground.runCode({json.dumps(FAILING)}).then(() => 0)")
time.sleep(2)
print("after the failing run:", shown())
page.browser("eval", f"playground.setCode({json.dumps(SLOW)}); document.getElementById('run').click(); 0")
time.sleep(0.3)
print("while the next run runs:", shown())
page.browser("close")
