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
- Open: registry as its own repository; version pinning (`url#ref`); an update command.
