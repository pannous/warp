# Handover: instance-words-build (warp-fixer, 2026-10-27, before the reboot)

## Done (on main, 1e8dcd512c)
- A lookup on a class instance reads its struct; its entries are never built for one lookup. Late-made words in
  core/emitter/instances.warp: is_class_instance, instance_place (byte count first, then bytes; extra fields after
  the declared ones), instance_field_at (extended past the declared fields into the extras). Runtime side:
  core/runtime/maps.warp map_index, has_fields, value_at (map_get, map_get_or), extra_place, extra_value_at.
- Measured: a stage-B compiler (its runtime this one) compiles compiler.warp in 86-111 s of CPU against 122 s for a
  compiler with the seed's runtime (before declared-class). Before this fix stage B took 2.6x the samples
  (instance_entries 27% inclusive).
- Checks on ee3823b16d: smoke (hello), self test (warp rebuild self, hello via warp self), gate tier 1 green (73 s).
  The final merge 1e8dcd512c (main's newest commits) went to main untested: a smoke test of main confirms it.

## Left
- The 131 class test files of test/types against main (data/t/classes in the worktree) did not finish before the
  reboot: run them again (`WARP_TEST_SLOTS=4 warp test <folder of copies>` on the branch's lib and on main's, compare
  the ✗ lines by file name), fix forward what is new.
- The seed hold (decision: no seed refresh until the instance runtime is fast) can be lifted once that comparison
  shows nothing new: tell the supervisor, warp-text-field and warp-samples.
- Tell warp-text-field that core/emitter/instances.warp and runtime_words.warp are free again.
- Profiles in /Volumes/Worktrees/instance-words-build/data/prof (b.json, b2.json, b3.json; summed by a script that
  counts self and inclusive samples per function of the Firefox Profiler JSON). Sample counts follow wall time, so
  only shares compare across runs; time with `/usr/bin/time -p` user seconds, alternating the two compilers.
- Reported to warp-text-field: Op·text is ~15% of a stage-B rebuild (node_equal per case), their area.

## Next in the queue (supervisor's order)
method-name-clash, then runtime-global-shadowing, then the run-time speed of compute samples (mandelbrot, sorting,
primes: time on main, profile the biggest cost).
