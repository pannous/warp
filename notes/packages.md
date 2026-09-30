# Packages: `use <name>` falls back to the registry

- `use name` (src/modules.rs): a local `name.wasp` in the search directories wins; else a builtin or FFI library; a name
  in the registry `packages.wasp` (repository root, compiled in with `include_str!`) is fetched by a shallow `git clone`
  into `packages/<name>` below the working directory (gitignored), once, and its `<name>.wasp` loaded like any module.
  A package without `<name>.wasp` only brings files.
- `module_directory`: compile-time text, the directory of the file it is written in ("." for the program), so a package
  reads its own files: uniscript.wasp has `read(module_directory + "/data/entities.idx")`. `read` itself is relative to
  the working directory at run time; a fetched package is below it, so the page preloads packages/uniscript/data/entities.idx.
- Concurrency: a process-wide mutex; parallel processes clone into `.<name>.fetching.<pid>` and rename, the loser deletes its copy.
- A local checkout stands in for a fetch: `ln -s ~/dev/uniscript packages/uniscript`. Update: `git -C packages/uniscript pull`.
- First package: uniscript (github.com/pannous/uniscript): its uniscript.wasp and data replaced warp's lib/uniscript.wasp
  and data/uniscript/. warp's tests/test_uniscript.rs still tests it (through `use uniscript`).
- Versions (src/versions.rs): `use x version 1.2.3` exactly, `use x from 1.2.3` / `use x >= 1.2.3` that or later.
  The default branch's module declares its version with a top level `version 1.2.3`; if it does not satisfy the
  requirement, the best git tag (`v1.2.3` or `1.2.3`, via `git ls-remote --tags`) is cloned into packages/<name>@<version>.
  A local module named in a versioned `use` must declare a satisfying version.
- `version` is a soft keyword (like Python's `match`): only before digits or a text; `version = 2` stays a variable.
  `1.2.3` (two dots or more) lexes as a version literal; `version 1.10` keeps 1.10. Versions compare part by part at
  compile time (`1.9 < version 1.10`, `1.2.0 == 1.2`), otherwise they are their text.
- Command line: `warp use uniscript >= 0.2` is just the program, fetching like any other.
- Open: registry as its own repository (later); an update command.
