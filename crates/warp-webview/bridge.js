// The js runtime of the native warp (decision native-js-webview): what `use js` calls in libwarp_webview's hidden web
// view, as web/playground/host-foreign.js's js runtime answers it in the page. warp_webview.m puts this file after
// web/playground/js-members.js (jsMember) into the page once, then calls warpForeignCall with the JSON of each call.
// Objects without a plain form (a Map, a Date, a function) stay here behind ids, the program holding their handles
// {$handle, type, text} as the page's host.js handleOf gives them
const warpHeld = [];
const warpHandleOf = value => ({ $handle: warpHeld.push(value), type: value?.constructor?.name ?? typeof value, text: String(value).slice(0, 200) });
const warpUnhandled = value => value !== null && typeof value === "object" && "$handle" in value ? warpHeld[value.$handle - 1] : value;
const warpIsPlainObject = value => [Object.prototype, null].includes(Object.getPrototypeOf(value));
// the value as JSON can carry it, as web/playground/values.js newGuestValue reads one
function warpPlain(value) {
	if (value === undefined) return null;
	if (typeof value === "function") return warpHandleOf(value);
	if (typeof value !== "object" || value === null) return value;
	if (Array.isArray(value) || ArrayBuffer.isView(value)) return Array.from(value, warpPlain);
	if (!warpIsPlainObject(value)) return warpHandleOf(value);
	return Object.fromEntries(Object.entries(value).map(([name, item]) => [name, warpPlain(item)]));
}
// the program's page after each render (lib/markup.warp page_render), so its elements by id (`who.focus()`) are here;
// what differs is changed (markup.js morphChildren), so a field keeps its focus and what is typed in it
function warpShowPage(html) {
	morphChildren(document.body, parsedHtml(html));
}
// each event of the page an element handles, {event: "click·1", detail} as markup.js elementEvent names it, to the
// native warp (warp_webview.m warpWindow.next), as the pages warp builds give theirs to the program (site.js)
const warpFieldDetail = happened => inputDetail(happened.composedPath()[0]);
const WARP_PAGE_EVENTS = { click: () => ({}), input: warpFieldDetail, ...Object.fromEntries(Object.keys(FIELD_EVENT_TYPES).map(event => [event, warpFieldDetail])) };
for (const [event, detail] of Object.entries(WARP_PAGE_EVENTS)) {
	document.addEventListener(FIELD_EVENT_TYPES[event] ?? event, happened => {
		const found = elementEvent(event, happened, detail(happened));
		if (found) window.webkit?.messageHandlers?.warpEvent?.postMessage(JSON.stringify(found));
	});
}
// `use js Math; Math.max(1, 5)`: the module's name or a handle, the member, the arguments (null: a read); a promise is
// awaited (callAsyncJavaScript)
async function warpForeignCall(request) {
	const { module, member, arguments: given } = JSON.parse(request);
	const value = typeof module === "object" ? warpUnhandled(module) : globalThis[module];
	if (value === undefined) throw new Error(`js ${module}.${member}: the web view has no global ${module}`);
	const label = typeof module === "string" ? module : "value";
	return JSON.stringify(warpPlain(await jsMember(value, label, member, given?.map(warpUnhandled) ?? null)));
}
