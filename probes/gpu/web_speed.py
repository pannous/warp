#!/usr/bin/env python3
"""card g_odW4: probes/gpu/render_speed.warp in the local playground (headless Chrome, web/playground/build.sh first)"""
import json, os, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "..", "web", "playground"))
import test_in_browser as page

page.serve(None)
page.open_page(f"http://127.0.0.1:{page.PORT}/web/playground/")
source = open(os.path.join(os.path.dirname(__file__), "render_speed.warp")).read()
time.sleep(3)
script = f"""(async () => {{ const report = await playground.evaluate({json.dumps(source)});
	return JSON.stringify({{ value: report.value, printed: report.printed }}); }})()"""
print(page.browser("eval", script))
page.browser("close")
