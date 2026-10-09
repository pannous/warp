#!/usr/bin/env python3
"""card g_oQnE: with an API key, a pause in typing at a line's end asks Claude for a completion by itself. With an
invalid key the real API refuses: its reason shows once under the line (headless Chrome, after build.sh). With
ANTHROPIC_API_KEY set the gray continuation itself is checked instead."""
import json, os, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "..", "web", "playground"))
import test_in_browser as page

KEY = os.environ.get("ANTHROPIC_API_KEY", "sk-ant-invalid-probe-key")
page.serve(None)
page.open_page(f"http://127.0.0.1:{page.PORT}/web/playground/")
time.sleep(3)
page.browser("eval", f"document.getElementById('auto').checked = false; localStorage.setItem('warp-playground-anthropic-key', {json.dumps(KEY)}); playground.setCode(''); 0")
page.browser("focus", ".CodeMirror textarea")
shown = lambda selector: json.loads(json.loads(page.browser("eval", f"JSON.stringify([...document.querySelectorAll('{selector}')].map(e => e.textContent))")))
page.browser("keyboard", "type", "squares = [1, 2, 3].map(x => x")
time.sleep(6)
said = shown(".completion") or shown(".completion-failed")
print("after a pause:", said)
page.browser("keyboard", "type", " ")
time.sleep(4)
print("after a second pause:", shown(".completion") or shown(".completion-failed"))
page.browser("eval", "localStorage.removeItem('warp-playground-anthropic-key'); 0")
page.browser("close")
sys.exit(0 if said else 1)
