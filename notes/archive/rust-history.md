# archive/rust-history

Old Rust-only branches, folded here so their commits stay reachable (user, 2026-10-24). Each was merged with -s ours: main's tree is unchanged; view a branch with `git show <tip>` or `git log <tip>`.

| branch | tip | verdict and purpose |
|---|---|---|
| cast-list | 5bb0dad9aa9ada9d844fbbe275d79b3405c4e403 | [dead] `[1, 2] as floats`, `xs = [] as list of int`: casts to list types; once merged, only the Rust part (src/lowering, src/analyzer, src/wasm_emitter casts) still differs, and src/ is frozen |
| code-empty | 75ca57a96bb7fa892e70044c023092e40727d511 | [dead] `{}` in code as the empty map (EMPTY_MAP_KIND) in the Rust emitter; wip, never built or tested, and src/ is frozen |
| function-value-read | 54b067d87613acb3efdbd67c90b93f8c9c62b002 | [dead] Rust lowering: nested function-list reads stay callable; agent gone, src/ frozen |
| gpu-multipass | 30febbd2e1c37056e32ac5333968e98d73a1d286 | [dead] native GPU render passes: `render(f, w, h)` draws f into an image another shader reads (src/graphics, Rust host); contained in gpu-multipass-web; parked until GPU work resumes, conflicts |
| gpu-multipass-web | aa910f90ba7c627c3bd45b126696c4033cf54724 | [dead] gpu-multipass plus the browser GPU host drawing render's images first; mostly Rust, 4000 commits behind, conflicts; parked until GPU work resumes |
| lib-json | ae56965513efe93793a7308e59d7a454379c3480 | [dead] notes only: to_json({}) gives null until `{}` is a runtime map (code-empty, dead) |
| path-written | 5731b5b4c80f25ce9bf55d7d198dad700f3e8dc5 | [dead] Rust: a path written without quotes stays a Path through parameters and results (option A); the user chose option B, path as its own type (decision path-type) |
| program-units | b200a7435a529d4ba9e9f31ef4afc2a745afae9e | [superseded] a plain `furlong := 201.168 m` defined a unit (Rust units, Oct 15); replaced on main by `unit` lines: a598aee3b, eb8a126b6, 3648ca299. Nothing left worth porting |
| reflection-bound | 878da5e293f142fdedbf74284bb86e7274ca0e5c | [superseded] Rust: type(x), dir(x), x.params of a local named like a function read the variable; covered on main by local-named-like (46be24416) |
| sin-says | cb80081971f747ab2a42a654c1b991fb688a5eb1 | [dead] Rust test pinning the diagnostic for assigning a libm word (sin = …); agent gone, src/ frozen |
