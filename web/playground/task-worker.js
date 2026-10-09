// A task of a program on its own Worker (host.js startTask, src/tasks.rs natively): the function runs in a fresh instance
// of the program's module, and its result (or failure) and printed output go back as JSON in the shared buffer the
// starting program waits on with Atomics.wait.

// a built site's task Worker gets the site's scripts as ?scripts=… (host-tasks.js addTaskWorker)
const SITE_SCRIPTS = new URL(self.location.href).searchParams.get("scripts")?.split(",");
importScripts(...(SITE_SCRIPTS ?? ["reader.js", "imports.js", "host.js"]));
if (!SITE_SCRIPTS) importScripts(...HOST_PART_FILES, "components.js", "served-files.js");
self.postMessage(TASK_WORKER_READY); // the pool takes this Worker only once it has loaded (host.js prepareTaskPool)

self.onmessage = ({ data }) => data.fetch ? fetchInto(data) : data.gpu ? gpuJobInto(data) : runTaskInto(data);

function runTaskInto({ module, name, ints, values, shared, arrays, captured, control, channels }) {
	let output = "";
	const hooks = { print: text => { output += text; }, panicked: text => { output += text; } };
	const record = runTask(module, hooks, [], name, ints, values, arrays, captured, control, channels);
	if (typeof record.value === "bigint") record.ints = true; // a function of Ints: its Int result as a tree
	writeShared(shared, { ...record, value: taskTree(record.value), output });
}

// `users := fetch url` of a running program (host.js startFetch): the reply goes into shared memory, which the program
// reads at its check points, and a message tells the program's Worker once it is back in its event loop
async function fetchInto({ fetch: url, body, shared }) {
	writeShared(shared, await fetchReplyOf(url, body));
	self.postMessage(FETCH_DONE);
}
