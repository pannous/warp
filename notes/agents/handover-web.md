# Handover: warp-web (2026-10-10)

Retired with no web or playground cards left; the next warp-web starts here.

## Open
- web-apis-rest (Later): the leftovers of web-apis, low value at sample sizes. These are clipboard read in a page
  (navigator.clipboard.readText is async, so it needs JSPI or a page round trip), more named buffers for gpu_compute
  (a map of named arrays), and gpu_render straight into a page canvas (GPUCanvasContext, with no pixel round trip).
  What exists is in the "web-apis" sections of notes/web_framework.md (frames, notify, WebGPU, WebIDL, WebSocket).
- Optional: tour-firefox in pages.yml still retries a stalled Firefox start. With page-made task workers
  (task-workers.js, notes/web_playground.md "tour-firefox-stall") the retry may be simplified once CI stays green.

## Browser probes
Browsers run in CI only (no Chrome or Firefox on the Mac). Each shell probe skips unless `CI` is set. To try one,
push a probe branch whose pages.yml step runs it, then dispatch `gh workflow run pages.yml --ref <branch>`
(see firefox-stall-probe-a/b/c).
- probes/firefox_start/starts.py: counts stalls per start stage over many fresh Firefox starts (`_site [loads]`)
- probes/firefox_hello_hang/, probes/firefox_worker/slow_server.py: slow or stalled worker.js and warp.wasm loads
- probes/web_playground.py, probes/playground_render/, probes/playground_two/: the playground page
- probes/site/, probes/site_worker.py, probes/lazy_routes/: built sites (routes, deep links, task Workers, route split)
- probes/route_data/, probes/server_routes/, probes/todo_app/: served apps and RPC (curl, plus a browser in CI)
- probes/music_go/check.sh: one sample in the deployed playground (Firefox driver)
The drivers are web/playground/test_in_browser.py and firefox_driver.mjs.
