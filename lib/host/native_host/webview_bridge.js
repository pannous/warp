// The js runtime of the native warp (decision native-js-webview): what `use js` calls in the native host module's hidden
// web view, as web/playground/host-foreign.js's js runtime answers it in the page. lib/host/native_host/webview.warp puts this
// file after web/playground/js-members.js (jsMember) into the page once, then calls warpForeignCall with the JSON of each call.
// Objects without a plain form (a Map, a Date, a function) stay here behind ids, the program holding their handles
// {$handle, type, text} as the page's host.js handleOf gives them
const warpHeld = [];
const warpHandleOf = value => ({ $handle: warpHeld.push(value), type: value?.constructor?.name ?? typeof value, text: String(value).slice(0, 200) });
// a function of the program ({$callback: slot}, webview.warp handed_over) as a JS function: each call asks the run
// synchronously at the page's own origin (webview.warp called_back), its arguments and result crossing as JSON
const warpCalledBack = slot => (...given) => {
	const request = new XMLHttpRequest();
	request.open("GET", `/${slot}?${encodeURIComponent(JSON.stringify(given.map(warpPlain)))}`, false);
	request.send();
	if (request.status !== 200) throw new Error(request.responseText);
	return JSON.parse(request.responseText);
};
const warpUnhandled = value => value === null || typeof value !== "object" ? value : "$handle" in value ? warpHeld[value.$handle - 1] : "$callback" in value ? warpCalledBack(value.$callback) : value;
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
// each event of the page its program hears (markup.js listenToElements: an element's, a key, a form sent), {event:
// "click·1", detail} as markup.js elementEvent names it, to the native warp (lib/host/native_host/webview.warp warpWindow.next), as the
// pages warp builds give theirs to the program (site.js)
listenToElements(document, found => found && window.webkit?.messageHandlers?.warpEvent?.postMessage(JSON.stringify(found)));
// `use js Math; Math.max(1, 5)`: the module's name or a handle, the member, the arguments (null: a read); a promise is
// awaited (callAsyncJavaScript)
async function warpForeignCall(request) {
	const { module, member, arguments: given } = JSON.parse(request);
	const value = typeof module === "object" ? warpUnhandled(module) : globalThis[module];
	if (value === undefined) throw new Error(`js ${module}.${member}: the web view has no global ${module}`);
	const label = typeof module === "string" ? module : "value";
	return JSON.stringify(warpPlain(await jsMember(value, label, member, given?.map(warpUnhandled) ?? null)));
}
