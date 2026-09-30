# Packages: `use package <name>`

- Registry: `packages.wasp` at the repository root, `name: "git url"`, compiled into warp (`include_str!`), so it works
  from any directory. Meant to move into a registry repository of its own once there are more packages.
- `use package uniscript` (src/modules.rs `fetch_package`): shallow `git clone` into `packages/<name>` below the working
  directory (gitignored), once; then loads `packages/<name>/<name>.wasp` if the package has one, else contributes nothing
  (a data package: lib/uniscript.wasp reads `packages/uniscript/data/entities.idx` at run time).
- Concurrency: a process-wide mutex; parallel processes clone into `.<name>.fetching.<pid>` and rename, the loser deletes its copy.
- A local checkout stands in for a fetch: `ln -s ~/dev/uniscript packages/uniscript`. Update: `git -C packages/uniscript pull`.
- First case: uniscript (github.com/pannous/uniscript) replaced the vendored pre-split copy in data/uniscript/.
- Open: no version pinning (a `#ref` in the url), no update command, run-time `read` paths are relative to the working
  directory, so a program run from elsewhere needs its own packages/ (the fetch happens at compile time, same directory).
