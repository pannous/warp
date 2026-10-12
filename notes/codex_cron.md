# Timed Codex worker lifecycle

`notes/agents/codex_worker.sh` starts a durable, named thread through the desktop app-server daemon so the run appears in the Codex app. The user's crontab invokes it at 02:35, 07:35, 12:35, 17:35 and 22:35.

## Lifecycle contract

- `codex_app_run.py` prints `started <thread> status=active ...` only after `thread/start` and `thread/name/set` succeed.
- `turn/completed` is authoritative. `completed` exits 0; `failed`, `interrupted`, a local deadline, and a lost app-server connection are distinct nonzero outcomes.
- While a turn is quiet, the runner polls `thread/read`. This closes the gap where persistence says a turn is interrupted but the subscribed connection never receives `turn/completed`.
- The default deadline is four and a half hours, shorter than the five-hour schedule. A deadline sends `turn/interrupt`, waits for the persisted terminal state, writes the final artifacts, and reaps the proxy.
- Every path atomically writes `<run>.last.md` and `<run>.result.json`. A failed or partial turn gets a lifecycle footer instead of masquerading as a successful final response.
- The wrapper owns start and end notices to `Warp Boss`. Its EXIT and signal traps preserve the artifact, remove the run lock, stop its exact child process, and send one end notice.
- Merge-request blocks are relayed only from a completed turn. A partial block from an interrupted or timed-out turn is never sent to the Integrator.

The `codex exec` fallback remains intentionally loud because it is not visible in the desktop app. It has the same deadline and finalization path.

## Verification

Run the real app-server probes, which create short visible Codex threads and keep their ignored evidence under `data/agents/codex/lifecycle-probe/`:

```sh
/usr/bin/python3 probes/codex_cron_lifecycle.py success
/usr/bin/python3 probes/codex_cron_lifecycle.py timeout
/usr/bin/python3 probes/codex_cron_lifecycle.py interrupt
/usr/bin/python3 probes/codex_cron_lifecycle.py disconnect
```

For the whole wrapper, set `CODEX_PROBE_PROMPT`; this uses `data/agents/codex/probe/` and its own lock while retaining real app-server and Warp Boss notice delivery:

```sh
CODEX_PROBE_PROMPT='Reply with just: ok' CODEX_TURN_TIMEOUT_SECONDS=60 notes/agents/codex_worker.sh
```

## Desktop project grouping limitation

The app-server `project/list` ids are internal state-database ids. Passing one as `thread/start.projectId` persisted that internal id but the desktop sidebar still reported no app project. The ordinary main-checkout run leaves `projectId` unset so desktop cwd grouping remains possible. Card `codex-app` tracks making this identity mapping explicit.

During a real probe whose turn was actively running `/bin/sleep`, app-server emitted `thread/status/changed: active`, while the desktop task listing reported `notLoaded`. Card `codex-desktop` tracks that product-level mismatch. Until it is fixed upstream, the ordered Warp Boss notices and `<run>.result.json` are the accurate active/terminal status record.
