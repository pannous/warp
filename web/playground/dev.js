// The page of `warp dev app.wasp` (src/dev_server.rs, card web-dev): asks the dev server for the build's version and
// failure; a new version reloads the page, a failure shows as an overlay (its position, the source line, its fix) over the
// page, which runs on with the last good build until the program builds again.

const DEV_STATE = "/wasp-dev/state";
const DEV_POLL_MILLISECONDS = 300;
const DEV_OVERLAY_ID = "wasp-dev-overlay";
const DEV_OVERLAY_STYLE = "position:fixed;inset:0;z-index:2147483647;margin:0;padding:2em;overflow:auto;" +
	"background:rgba(20,20,20,.92);color:#ffb4b4;font:14px/1.5 ui-monospace,Menlo,monospace;white-space:pre-wrap";

let devVersion = null; // the version this page shows

async function pollDev() {
	try {
		const { version, error } = await (await fetch(DEV_STATE)).json();
		if (error) showDevError(error);
		else if (devVersion !== null && version !== devVersion) return location.reload();
		else document.getElementById(DEV_OVERLAY_ID)?.remove();
		devVersion ??= version;
	} catch {
		// the dev server restarts: ask again
	}
	setTimeout(pollDev, DEV_POLL_MILLISECONDS);
}

function showDevError(error) {
	const overlay = document.getElementById(DEV_OVERLAY_ID) ?? document.body.appendChild(Object.assign(document.createElement("pre"), { id: DEV_OVERLAY_ID }));
	overlay.style.cssText = DEV_OVERLAY_STYLE;
	overlay.textContent = `warp dev: ${error}`;
}

pollDev();
