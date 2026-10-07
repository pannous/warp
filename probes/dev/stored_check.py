# web-stores in the playground after the kept values moved to markup.js (card web-dev): a stored value counts across
# reloads of the page (localStorage); run via tests/queue.sh
import json, sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "..", "web", "playground"))
import test_in_browser as page

CODE = "stored probe_visits = 0\nprobe_visits += 1\nprobe_visits"
server = page.serve(None)
url = f"http://127.0.0.1:{page.PORT}/web/playground/"
script = """(async () => {
	const settled = async () => { while (!document.getElementById("value").textContent || document.getElementById("status").textContent === "running…") await new Promise(done => setTimeout(done, 50)); };
	await settled();
	playground.setCode(%s);
	await playground.evaluate();
	await new Promise(done => setTimeout(done, 300));
	await settled();
	return JSON.stringify([document.getElementById("value").textContent, localStorage.getItem("wasp stored probe_visits")]);
})()""" % json.dumps(CODE)
seen = []
for visit in range(2):
	page.open_page(url)
	page.wait_for_isolation()
	if visit == 0:
		page.browser("eval", "localStorage.removeItem('wasp stored probe_visits')")
	seen.append(json.loads(json.loads(page.browser("eval", script))))
print(seen)
page.browser("close")
server.shutdown()
