# Known failures (Rust cleanup phase)
During the Rust cleanup (notes/rust_cleanup.md, card rust-cleanup) a cleanup branch may break up to 10% of the tests
when they are edge cases, never core behaviour. The branch itself adds one line per broken test below, in the same
commit series: the test's full path, the card, a one-line reason. The gate (push_tested.sh) accepts a failure listed
here at the tested tip, so later branches aren't blamed for it; a failure not listed stops the push and the branch is
held. A known failure that passes again: delete its line in the branch that fixes it. On demand the Integrator turns
the lines into cards for the stage's worker.

Format: `- <module::path::test_name> | <card> | <reason>`

- wasm::test_gc_struct::test_raw_person_struct | rust-emitter | the test reads a raw struct's text field as `$String(ptr, len)`; texts are `$Text` GC byte arrays now (test edit implied by the layout: read the array)
- wasm::test_gc_struct::test_class_instance_raw | rust-emitter | same: reads the raw text field as `$String(ptr, len)`, now a `$Text` byte array
