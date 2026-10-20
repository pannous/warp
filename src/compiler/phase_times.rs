//! Where a run's time goes (card prelude-cache-runs): `WARP_PHASE_TIMES=1 warp x.warp` lists on stderr each phase's
//! wall time, summed over its calls. A phase inside another counts in both (resolve is part of lowering); a phase
//! calling itself counts once
use std::collections::BTreeMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

pub(crate) const SETTING: &str = "WARP_PHASE_TIMES";

static ON: LazyLock<bool> = LazyLock::new(|| crate::setting(SETTING).is_some());
/// Per phase: its total time, its calls and how many are running now
static PHASES: Mutex<BTreeMap<&'static str, (Duration, usize, usize)>> = Mutex::new(BTreeMap::new());
static STARTED: LazyLock<Instant> = LazyLock::new(Instant::now);

/// `work`, its time counted under `phase` when WARP_PHASE_TIMES is set
pub fn timed<T>(phase: &'static str, work: impl FnOnce() -> T) -> T {
	if !*ON {
		return work();
	}
	LazyLock::force(&STARTED);
	let outermost = change(phase, |(_, calls, running)| {
		*running += 1;
		*calls += usize::from(*running == 1);
		*running == 1
	});
	let start = Instant::now();
	let result = work();
	let elapsed = start.elapsed();
	change(phase, |(total, _, running)| {
		*running -= 1;
		if outermost {
			*total += elapsed;
		}
	});
	result
}

/// timed under a phase named at run time, `name` called only when WARP_PHASE_TIMES is set
pub fn timed_as<T>(name: impl FnOnce() -> String, work: impl FnOnce() -> T) -> T {
	match *ON {
		true => timed(Box::leak(name().into_boxed_str()), work),
		false => work(),
	}
}

fn change<T>(phase: &'static str, update: impl FnOnce(&mut (Duration, usize, usize)) -> T) -> T {
	let mut phases = PHASES.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	update(phases.entry(phase).or_default())
}

/// The phases' times on stderr, longest first, when WARP_PHASE_TIMES is set
pub fn report() {
	if !*ON {
		return;
	}
	let phases = PHASES.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
	let mut rows: Vec<_> = phases.iter().collect();
	rows.sort_by_key(|(_, (total, _, _))| std::cmp::Reverse(*total));
	eprintln!("phase times ({} ms since the first phase):", STARTED.elapsed().as_millis());
	for (phase, (total, calls, _)) in rows {
		eprintln!("{:>8.1} ms  {phase} ({calls}×)", total.as_secs_f64() * 1000.0);
	}
}
