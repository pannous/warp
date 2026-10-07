# web-transitions: a removed item is marked leaving while it fades, then gone; run via tests/queue.sh
import json, sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "..", "web", "playground"))
import test_in_browser as page

server = page.serve(None)
page.open_page(f"http://127.0.0.1:{page.PORT}/web/playground/")
page.wait_for_isolation()
script = """(async () => {
	await playground.chooseExample("transitions");
	while (document.getElementById("status").textContent === "running…") await new Promise(done => setTimeout(done, 50));
	const root = document.getElementById("rendered").shadowRoot;
	const pause = ms => new Promise(done => setTimeout(done, ms));
	const snapshot = () => [...root.querySelectorAll("li")].map(li => li.textContent + (li.hasAttribute("data-wasp-leaving") ? "(leaving)" : ""));
	[...root.querySelectorAll("button")].find(button => button.textContent === "remove").click();
	let during = [];
	for (let tries = 0; tries < 40 && !during.some(text => text.includes("leaving")); tries++) { await pause(10); during = snapshot(); }
	await pause(400);
	return JSON.stringify({ during, after: snapshot() });
})()"""
print(json.loads(page.browser("eval", script)))
page.browser("close")
server.shutdown()
