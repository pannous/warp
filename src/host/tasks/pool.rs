//! The fixed pool of native workers the tasks of one program run on
use super::*;

/// Tasks may block in sleep, channels and host calls, so one worker per core would leave cores idle.
pub(crate) const TASK_WORKERS_PER_CORE: usize = 2;

pub(super) struct TaskJob {
	pub(super) id: i64,
	pub(super) function: String,
	pub(super) arguments: Vec<TaskValue>,
	pub(super) captured: Vec<(String, Captured)>,
	pub(super) control: Arc<TaskControl>,
	pub(super) voice: crate::sound::Voice,
}

pub(super) struct TaskPoolState {
	jobs: Mutex<VecDeque<TaskJob>>,
	ready: Condvar,
	stopping: AtomicBool,
}

pub(super) struct TaskPool {
	state: Arc<TaskPoolState>,
	workers: Mutex<Vec<JoinHandle<()>>>,
}

thread_local! {
	pub(super) static IN_TASK_WORKER: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

impl TaskPool {
	pub(super) fn new() -> Self {
		Self {
			state: Arc::new(TaskPoolState {
				jobs: Mutex::new(VecDeque::new()),
				ready: Condvar::new(),
				stopping: AtomicBool::new(false),
			}),
			workers: Mutex::new(Vec::new()),
		}
	}

	pub(super) fn start(&self, table: Weak<TaskTable>) -> Result<()> {
		let mut workers = self.workers.lock().expect("task workers");
		for number in 0..worker_count() {
			let state = self.state.clone();
			let table = table.clone();
			let worker = std::thread::Builder::new().name(format!("warp-task-{number}")).spawn(move || {
				IN_TASK_WORKER.with(|inside| inside.set(true));
				while let Some(job) = TaskPool::next(&state, true) {
					let Some(table) = table.upgrade() else { break };
					table.run_job(job);
				}
			})?;
			workers.push(worker);
		}
		Ok(())
	}

	pub(super) fn submit(&self, job: TaskJob) {
		self.state.jobs.lock().expect("task queue").push_back(job);
		self.state.ready.notify_one();
	}

	pub(super) fn help(&self) -> Option<TaskJob> {
		TaskPool::next(&self.state, false)
	}

	fn next(state: &TaskPoolState, wait: bool) -> Option<TaskJob> {
		let mut jobs = state.jobs.lock().expect("task queue");
		loop {
			if let Some(job) = jobs.pop_front() {
				return Some(job);
			}
			if !wait || state.stopping.load(Ordering::Relaxed) {
				return None;
			}
			jobs = state.ready.wait(jobs).expect("task queue");
		}
	}
}

impl Drop for TaskPool {
	fn drop(&mut self) {
		self.state.stopping.store(true, Ordering::Relaxed);
		self.state.ready.notify_all();
		let current = std::thread::current().id();
		for worker in self.workers.get_mut().expect("task workers").drain(..) {
			if worker.thread().id() != current {
				let _ = worker.join();
			}
		}
	}
}

/// The fixed number of native workers used by one program run.
pub fn worker_count() -> usize {
	std::thread::available_parallelism().map_or(1, |cores| cores.get() * TASK_WORKERS_PER_CORE)
}

/// The machine's cores: a parallel let forks while fewer tasks run (task_forks)
pub(crate) fn cores() -> i64 {
	worker_count() as i64
}
