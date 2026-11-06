// A helper Worker of a program's pool (host-tasks.js addTaskWorker): it fetches and runs GPU jobs for the program's
// Worker, which cannot await while it runs, and answers as JSON in the shared buffer it waits on with Atomics.wait.

importScripts("imports.js", "js-members.js", "std.js", "host.js");
importScripts(...HOST_PART_FILES, "components.js", "served-files.js");
// the program's Worker, through the port the page gives with the first message (task-workers.js serveTaskWorkers)
let program;
self.onmessage = ({ data }) => {
	if (data.paintCanvas) return paintCanvasArrived(data.paintCanvas); // host-gpu.js, from the page
	program = data.port;
	program.onmessage = ({ data }) => data.fetch ? fetchInto(data) : gpuJobInto(data);
	program.postMessage(TASK_WORKER_READY); // the pool takes this Worker only once it has loaded (host-tasks.js prepareTaskPool)
};

// a promise failing without a reason (Firefox's WebGPU, card uncaught-exception-undefined) is said with where it failed,
// not as the console's bare "uncaught exception: undefined"
self.addEventListener("unhandledrejection", event => {
	if (event.reason !== undefined) return;
	event.preventDefault();
	console.error("warp task Worker: a browser API failed without saying why", event.promise);
});


// `users := fetch url` of a running program (host.js startFetch): the reply goes into shared memory, which the program
// reads at its check points, and a message tells the program's Worker once it is back in its event loop
async function fetchInto({ fetch: url, body, shared }) {
	writeShared(shared, await fetchReplyOf(url, body));
	program.postMessage(FETCH_DONE);
}
