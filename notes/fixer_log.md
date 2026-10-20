- 2026-10-19 fixnow-builtin-function-add: a list given to a function value (closure_call_N) is held as Nodes, as the call may change it; main's typed-list pass had only kept it a Node list by seeing builtin·add's own `xs += [x]` (list_sharing.rs Holders)
- 2026-10-20 guide-leak: the guide test runs its snippets in a scratch working folder: with_working_folder (compile context working_folder, read by the file words, read and pwd) for in-process evaluate, printed_in (current_dir) for the CLI; no rewriting of call forms
- 2026-10-20 precedence-warning: parser.warp type_declaration says `(a and b) or not c` with its parentheses, the and/or mix warning gone from every self-hosted run (the only lib/compiler one)
- 2026-10-20 fast-decimals: fast code reads its decimals as floats (use_statements.rs machine_numbers, was float_division): `fast` then `0.5 + 0.25` gave 0, the exact ratio taken for a machine int
||||||| fe2acb6ee8
