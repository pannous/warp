#!/usr/bin/env python3
"""card g_oQgw: the editor completes words, `use` modules and uniscript entities, and turns a typed <:name> into its
character (headless Chrome, after web/playground/build.sh)"""
import json, os, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "..", "web", "playground"))
import test_in_browser as page

page.serve(None)
page.open_page(f"http://127.0.0.1:{page.PORT}/web/playground/?no_auto")
time.sleep(3)
page.browser("eval", "document.getElementById('auto').checked = false; playground.setCode(''); 0")
page.browser("focus", ".CodeMirror textarea")
listed = lambda: json.loads(json.loads(page.browser("eval", "JSON.stringify([...document.querySelectorAll('.completions:not([hidden]) li')].map(li => li.textContent))")))
code = lambda: json.loads(page.browser("eval", "playground.code()"))
failures = []
def expect(what, got, wanted):
	print(f"{'ok  ' if got == wanted else 'FAIL'} {what}: {got!r}" + ("" if got == wanted else f", wanted {wanted!r}"))
	failures.extend([what] * (got != wanted))

def typed(text, settle=0.4):
	page.browser("keyboard", "type", text)
	time.sleep(settle)

typed("pri")
expect("a word lists its completions", "print" in listed(), True)
page.browser("press", "Tab")
expect("Tab takes the first", code(), "print")
typed(' "\\:alph')
expect("an entity lists its character", listed()[:1], ["alphaα"])
page.browser("press", "Tab")
expect("Tab takes the entity's character", code(), 'print "α')
typed(" <:beta>")
expect("a typed tag becomes its character", code(), 'print "α β')
typed('"\nuse gra')
expect("use lists the modules", listed(), ["graphics"])
page.browser("press", "Escape")
expect("Esc closes", listed(), [])
typed("\npri")
page.browser("press", "Enter")
expect("Enter before choosing is a new line", code().split("\n")[-2:], ["pri", ""])
typed("co")
second = listed()[1]
page.browser("press", "ArrowDown")
page.browser("press", "Enter")
expect("Enter takes a chosen one", code().split("\n")[-1], second)
page.browser("close")
sys.exit(1 if failures else 0)
