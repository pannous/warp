// The page of `warp dev app.warp` (src/web/dev_server.rs, card web-dev): asks the dev server for the build's version and
// failure; a new version reloads the page, a failure shows as an overlay (its position, the source line, its fix) over the
// page, which runs on with the last good build until the program builds again.

const DEV_STATE = "/warp-dev/state";
const DEV_POLL_MILLISECONDS = 300;
const DEV_OVERLAY_ID = "warp-dev-overlay";
const DEV_OVERLAY_STYLE = "position:fixed;inset:0;z-index:2147483647;margin:0;padding:2em;overflow:auto;" +
	"background:rgba(20,20,20,.92);color:#ffb4b4;font:14px/1.5 ui-monospace,Menlo,monospace;white-space:pre-wrap";
// the failing word as the playground's editor marks it (playground.css .marked-error)
const DEV_MARK_STYLE = "text-decoration:underline wavy #ff7b6e 2px;text-decoration-skip-ink:none;text-underline-offset:3px;" +
	"background:rgba(255,123,110,.14)";
// an excerpt of src/diagnostics/diagnostic.rs excerpt: `  2 | source` over `    |   ^^^`
const EXCERPT_SOURCE = /^ +\d+ \| /;
const EXCERPT_CARETS = /^ +\| ([\t ]*)(\^+)$/;

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

// the failure as texts and marks: each excerpt's source line with the word under its carets marked, the message (the
// failure's first line) on hover, as the playground's editor marks it (playground.js markWord); the caret line goes
function devErrorPieces(error) {
	const lines = error.split("\n");
	const pieces = [];
	let text = "";
	lines.forEach((line, index) => {
		const carets = lines[index + 1]?.match(EXCERPT_CARETS);
		if (EXCERPT_CARETS.test(line) && EXCERPT_SOURCE.test(lines[index - 1] ?? "")) return;
		if (index > 0) text += "\n";
		if (!EXCERPT_SOURCE.test(line) || !carets) return void (text += line);
		const characters = Array.from(line);
		const start = line.match(EXCERPT_SOURCE)[0].length + Array.from(carets[1]).length;
		const end = start + carets[2].length;
		pieces.push(text + characters.slice(0, start).join(""), { mark: characters.slice(start, end).join(""), message: lines[0] });
		text = characters.slice(end).join("");
	});
	return [...pieces, text].filter(piece => piece !== "");
}

function showDevError(error) {
	const overlay = document.getElementById(DEV_OVERLAY_ID) ?? document.body.appendChild(Object.assign(document.createElement("pre"), { id: DEV_OVERLAY_ID }));
	overlay.style.cssText = DEV_OVERLAY_STYLE;
	const marked = ({ mark, message }) => Object.assign(document.createElement("span"), { textContent: mark, title: message, style: DEV_MARK_STYLE });
	overlay.replaceChildren("warp dev: ", ...devErrorPieces(error).map(piece => typeof piece === "string" ? piece : marked(piece)));
}

// a page, not a check of devErrorPieces under node (tests/web/test_dev_marks.rs)
if (typeof document !== "undefined") pollDev();
