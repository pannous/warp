const SITE_ROOT = "warp-root";
const SITE_MODULE = "app.wasm";
const PAGE_HTML = "page·html";
const SITE_EVENTS = { click: () => ({}), input: happened => inputDetail(happened.composedPath()[0]) };
let site; // the program's run (host.js runProgram's holder)
const siteHooks = {
pagePath: () => location.pathname,
instantiated: holder => { site = holder; },
print: (text, stream) => (stream === 2 ? console.error : console.log)(text.replace(/\n$/, "")),
listen: holder => {
if (holder.timers) startTimers(holder, handler => showAfter(runTimer(holder, siteHooks, handler)));
},
arrived: (holder, handler) => holder === site && showAfter(runTimer(holder, siteHooks, handler)),
};
function showAfter(outcome) {
if (outcome.result === undefined) {
site.stopTimers?.();
return console.error("warp:", outcome.failure ?? outcome.trap ?? outcome.error);
}
show();
}
function show() {
const render = site.exports[PAGE_HTML];
if (!render) return console.error(`warp: app.wasm exports no ${PAGE_HTML}: build it with warp build --site`);
const template = document.createElement("template");
template.innerHTML = plainOfTree(readNode(site.exports, render()));
morphChildren(document.getElementById(SITE_ROOT), template.content);
}
async function hydrate() {
const bytes = new Uint8Array(await (await fetch(SITE_MODULE)).arrayBuffer());
Object.assign(storedValues, keptValues());
Object.assign(sessionValues, keptValues([SESSION_STORE]));
await self.loadDatabase?.();
self.keepStored = keepValue;
const holder = instantiateProgram(bytes, siteHooks);
if (holder.failure) return console.error("warp:", holder.failure);
await globalThis.loadRouteModule?.(holder);
const outcome = runMain(holder, siteHooks);
if (outcome.result === undefined) return console.error("warp:", outcome.failure ?? outcome.trap ?? outcome.error);
globalThis.followSiteLinks?.(goTo);
if (site) showAfter(outcome);
listenToElements(found => found && site.exports[`on·${found.event}·node`] && showAfter(runPageEvent(site, siteHooks, found.event, found.detail)));
}
function listenToElements(handle) {
for (const [event, detail] of Object.entries(SITE_EVENTS)) {
document.getElementById(SITE_ROOT).addEventListener(event, happened => handle(elementEvent(event, happened, detail(happened))));
}
}
async function goTo(path) {
navigate(site, siteHooks, path);
await loadRouteModule(site);
show();
focusRoute(document.getElementById(SITE_ROOT));
}
(globalThis.startSiteWorker ?? hydrate)();
