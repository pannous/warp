// Timers in the page (a part of host.js, which says how parts work; crates/warp-runtime/src/system_signals.rs): main
// records them, the page starts them after it (startTimers, in the playground's worker.js and a built site's site.js); a
// host without a page that stays (test-worker.js) only warns

const TIMER_HANDLER_PREFIX = "on·every·";
function addTimer(holder, hooks, id, timer, written) {
	if (!hooks.listen) return holder.warnings.push(`${written}: timers do not run here`);
	(holder.timers ??= []).push({ id: Number(id), handler: TIMER_HANDLER_PREFIX + id, ...timer });
}

// milliseconds from now to the next local minute_of_day on a weekday of the mask (bit 0 Sunday), as seconds_until_on
const EVERY_DAY = 0b1111111;
const DAY_MILLISECONDS = 24 * 3600 * 1000;
function millisecondsUntil(minuteOfDay, weekdays = EVERY_DAY, now = new Date()) {
	const midnight = new Date(now.getFullYear(), now.getMonth(), now.getDate());
	for (let daysAhead = 0; daysAhead <= 7; daysAhead++) {
		const due = new Date(midnight.getFullYear(), midnight.getMonth(), midnight.getDate() + daysAhead, 0, minuteOfDay);
		if (due > now && (weekdays || EVERY_DAY) & (1 << due.getDay())) return due - now;
	}
	return 7 * DAY_MILLISECONDS;
}

// a run's timers started: `fire(handler)` runs the handler on·every·<id> of each when it is due; stopTimers ends them
function startTimers(holder, fire) {
	const handles = [];
	const atClock = timer => handles.push(setTimeout(() => {
		fire(timer.handler);
		if (!timer.once) atClock(timer);
	}, millisecondsUntil(timer.minute, timer.weekdays)));
	for (const timer of holder.timers ?? []) {
		if (timer.every !== undefined) handles.push(setInterval(() => fire(timer.handler), timer.every));
		else atClock(timer);
	}
	holder.stopTimers = () => handles.forEach(handle => { clearTimeout(handle); clearInterval(handle); });
}

// what a timer says in the page: "every 500 ms", "at 09:00", "every day at 09:00", or the channel its listener reads
function timerLabel(holder, { id, every, minute, once }) {
	const channel = holder.channels?.get(id);
	if (channel) return `message from "${channel.name}"`;
	if (every !== undefined) return every % 1000 ? `every ${every} ms` : `every ${every / 1000} s`;
	const time = `${String(Math.floor(minute / 60)).padStart(2, "0")}:${String(minute % 60).padStart(2, "0")}`;
	return once ? `at ${time}` : `every day at ${time}`;
}

addHostPart({
	words: (holder, hooks) => ({
		signal_every: (id, milliseconds) => addTimer(holder, hooks, id, { every: Number(milliseconds) }, "on every …"),
		signal_daily: (id, minute, weekdays) => addTimer(holder, hooks, id, { minute: Number(minute), weekdays: Number(weekdays) }, "on every day at …"),
		signal_at: (id, minute) => addTimer(holder, hooks, id, { minute: Number(minute), once: true }, "at 9:00 {…}"),
	}),
});
