// Child process groups the web tests start (wrangler dev with its esbuild service and workerd): stopped whole, also
// when the test dies by a signal. A wrangler killed alone left `esbuild --service --ping` orphans that each spun a CPU
// core for days (user, 2026-10-13). Spawn the child with `detached: true` so it leads its own group.

const groups = new Set();

export function stopGroup(child) {
	groups.delete(child);
	try {
		process.kill(-child.pid, "SIGTERM");
	} catch {} // the group is gone already
}

export function stopGroupOnExit(child) {
	groups.add(child);
}

const stopAll = () => [...groups].forEach(stopGroup);
process.on("exit", stopAll);
for (const signal of ["SIGINT", "SIGTERM", "SIGHUP"]) {
	process.on(signal, () => {
		stopAll();
		process.exit(128 + { SIGINT: 2, SIGTERM: 15, SIGHUP: 1 }[signal]);
	});
}
