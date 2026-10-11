# Headless wait cleanup

- Keep the existing behavioral test: a 30-second queued sound must not make `wait_sound` wait in headless mode.
- Measure the `wait_sound` call inside the Warp program, excluding child compilation and synthesis; lower only the
  program's sample rate so the audio-clock duration remains 30 seconds with few samples.
- Run the focused test before and after the change, record the finding in `notes/sound.md`, then run `cargo fix --offline --allow-dirty --lib --bins`.
- Leave production sound behavior and unrelated tests unchanged.
