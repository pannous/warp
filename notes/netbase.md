# `use netbase` (card netbase-package, samples/netbase.wasp)

```wasp
use netbase
load wikidata
results = query "all cities with population > 100000"
print first 10 of results
debug results where country is germany
```

## What pannous/netbase is (surveyed 2026-10-07)
- A C++ semantic graph database (github.com/pannous/netbase, local checkout ~/dev/netbase, last commit 2023-04-22).
  Its data (Wikidata, Freebase, DBpedia, WordNet: ~600M statements) is imported into **shared memory**
  (`./netbase :import wikidata`, the increase-shared-memory.sh scripts), then served over HTTP by `./netbase :server`
  (port 81 by default, webserver.cpp).
- The query language is already natural: query.cpp rewrites `with/which/who/that/of/in` to `where`, so
  `all cities with population > 100000` is netbase's own syntax. HTTP: `/<format>/<query>`, format json, csv, xml,
  txt, html… (`/json/query/<q>`, `/json/all/<q>`, `?q=`).
- Clients exist for Python (netbase-python), Ruby, Node (node-netbase), Java; ~/dev/apps/netbase_wasm is an old
  emscripten build (netbase.bc / netbase.js), with no data.
- **No server is running**: netbase.pannous.com answers 301 → https with an empty body, and no netbase process runs on
  pannous.com. The local data folder `import/` is empty (0 B) apart from small samples (cities1000.txt, facts.n3…).

## So it needs a server: the plan
1. **The package (small, warp side)**: a repository `pannous/netbase-wasp` (or a `netbase.wasp` in pannous/netbase),
   listed in warp's `packages.wasp`, made of wasp code over what warp already has (`fetch`, `json`):
   - `netbase_server = env("NETBASE") or "https://netbase.pannous.com"`
   - `query(q) := parse_json(fetch(netbase_server + "/json/query/" + q))` (lib/json, lib/os `env`; URL-encoding the query is a missing small word): a list of records
     (id, name, statements as fields), so `first 10 of results` and `results where it.country == "germany"` are plain
     list words.
   - `load wikidata`: a request to the server to have that dataset (it is the server's import; a public server already
     has it loaded), else the error naming the server.
   No new compiler feature, except perhaps the phrases `first 10 of results` and `results where country is germany`
   (`where` with a bare field name meaning `it.country`): small undoable defaults, separate cards.
2. **The server (big, outside warp)**: someone has to run netbase with Wikidata loaded. Options:
   (a) bring netbase.pannous.com back: build on pannous.com, import Wikidata (tens of GB of shared memory, hours);
   (b) a small public dataset (cities1000.txt + wordnet) on a local `./netbase :server` for tests and the sample;
   (c) skip netbase and back the package with Wikidata's public SPARQL endpoint (query.wikidata.org), translating
       `all cities with population > 100000` into SPARQL: no server to run, but the natural-query translation
       would have to be rewritten in wasp.
   Recommended: (b) for tests now, (a) or (c) as the user decides.
3. Tests: against a local server started by the test (like test_web_server), never a mock.

## Open (for the user via the Interviewer)
- Which backend: revive netbase.pannous.com (a), a local small netbase (b), or Wikidata SPARQL (c)?
- Where the package lives: its own repository netbase-wasp, or netbase.wasp inside pannous/netbase?

## Built so far (default (b) of warp-03, 2026-10-07)
- `lib/extra/netbase.wasp`: `query(q)` / `query_at(server, q)` fetch `<server>/json/query/<q>` and parse the JSON; the server is
  env NETBASE, else http://localhost:8181. Without a server the error names the URL (tests/modules/test_netbase_package.rs).
  `load wikidata` is not defined: loading is the server's import.
- Writing it found two compiler bugs, fixed: `f() := a or "d"` (inference of or/and) and fetch of a computed URL.
- The local server is NOT running yet. ~/dev/netbase's arm64 binary (2023) runs, but:
  - the SysV shared-memory mode needs kern.sysv.shmmax above macOS's 4 MB (even the context segment asks 8 MB):
    `sudo sysctl -w kern.sysv.shmmax=4294967296 kern.sysv.shmall=1048576` (the user's sudo; not persistent);
  - the file mmap mode (`USE_MMAP=1` in netbase.config) only loads an existing import ("WRONG IMPORT" on fresh files)
    and never grows its files on macOS (file_allocate is a no-op there: SIGBUS on 0-byte maps);
  - the binary puts its data in `<binary dir>/data/` whatever data_path says: run a copy (scratch/netbase/netbase).
  - Docker (netbase.docker/) would need a Linux build of the C++ sources.
  Recommended next step: the user runs the sysctl line once, then `./netbase :import cities` and `./netbase :server`
  from a copy in scratch/netbase with a small netbase.config (SERVER_PORT=8181, maxNodes=2000000).
