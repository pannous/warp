// The page's side of a program Worker's task pool (host-tasks.js addTaskWorker): the page makes the task Workers, since in
// Firefox a Worker's own `new Worker` waits for the page's thread and, right after the browser started, that wait
// sometimes never ends (card tour-firefox-stall: 4 fresh starts in 80 stalled, probes/firefox_start/starts.py). The
// program's Worker asks {taskWorker: {id, url}} and gets {taskWorker: id, port}, a MessagePort to the new Worker;
// {endTaskWorker: id} stops one. The program's Worker, terminated, takes its task Workers with it as before.

// call before setting programWorker.onmessage: these messages are answered here and reach no other listener
function serveTaskWorkers(programWorker) {
	const taskWorkers = new Map();
	programWorker.addEventListener("message", event => {
		const { taskWorker, endTaskWorker } = event.data ?? {};
		if (taskWorker === undefined && endTaskWorker === undefined) return;
		event.stopImmediatePropagation();
		if (endTaskWorker !== undefined) {
			taskWorkers.get(endTaskWorker)?.terminate();
			return taskWorkers.delete(endTaskWorker);
		}
		const made = new Worker(taskWorker.url);
		const channel = new MessageChannel();
		made.postMessage({ port: channel.port1 }, [channel.port1]);
		programWorker.postMessage({ taskWorker: taskWorker.id, port: channel.port2 }, [channel.port2]);
		taskWorkers.set(taskWorker.id, made);
	});
	const terminate = programWorker.terminate.bind(programWorker);
	programWorker.terminate = () => {
		taskWorkers.forEach(made => made.terminate());
		terminate();
	};
	return programWorker;
}

// `clipboard` in a Worker (host-tasks.js askPage): the page's text into the Worker's shared buffer
const pasteInto = shared => answerInto(shared, async () => ({ text: await navigator.clipboard.readText() }));

// the page's answer to a waiting Worker, or {error}, as JSON in its shared buffer: host-tasks.js writeShared's layout
// ([state, length] as Int32, then the JSON), the state set last for the Worker waiting on it
async function answerInto(shared, answer) {
	const record = await answer().catch(failure => ({ error: failure.message ?? String(failure) }));
	const json = new TextEncoder().encode(JSON.stringify(record));
	const header = Int32Array.BYTES_PER_ELEMENT * 2;
	if (header + json.length > shared.byteLength) shared.grow(header + json.length);
	new Uint8Array(shared, header, json.length).set(json);
	const state = new Int32Array(shared, 0, 2);
	state[1] = json.length;
	Atomics.store(state, 0, 1);
	Atomics.notify(state, 0);
}
