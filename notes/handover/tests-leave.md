# tests-leave

The sound overload tests run with `test/functions` as their current directory and leave two WAVs there. The stored baseline has four passing cases. A direct original signal example returned `yes` and produced `chord_signal.wav` in the isolated worktree. The card explicitly permits changing the tests' output paths; their expected values stay unchanged. Default: per-process names and explicit cleanup so simultaneous tests cannot collide or overwrite old recordings. Next: verify cleanup using real recordings, then update the two cases.
