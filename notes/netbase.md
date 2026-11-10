# `use netbase` (card netbase-package, lib/extra/examples/netbase.warp)

```warp
use netbase
load wikidata
results = query "all cities with population > 100000"
print first 10 of results
debug results where country is germany
```

## What pannous/netbase is
- A C++ semantic graph database (github.com/pannous/netbase, local checkout ~/dev/netbase): Wikidata, Freebase,
  DBpedia, WordNet imported into shared memory (`./netbase :import wikidata`), served over HTTP by `./netbase :server`.
- **No server is running** (netbase.pannous.com answers an empty 301), and the local data folder `import/` holds only
  small samples (cities1000.txt, facts.n3…). The example stops at `load wikidata` (card netbase-wikidata-load).

## Open: a server
Someone has to run netbase with data loaded: (a) revive netbase.pannous.com (build on pannous.com, import Wikidata:
tens of GB of shared memory, hours); (b) a small dataset (cities1000.txt + wordnet) on a local `./netbase :server`
for tests and the sample; (c) skip netbase and translate the natural queries to Wikidata's SPARQL endpoint in warp.
Tests run against a local server started by the test, never a mock. Also open: where the package lives (its own
repository netbase-warp, or netbase.warp inside pannous/netbase), and the phrases `first 10 of results`,
`results where country is germany`.

Running ~/dev/netbase's arm64 binary (2023) locally:
- the SysV shared-memory mode needs kern.sysv.shmmax above macOS's 4 MB (even the context segment asks 8 MB):
  `sudo sysctl -w kern.sysv.shmmax=4294967296 kern.sysv.shmall=1048576` (the user's sudo; not persistent);
- the file mmap mode (`USE_MMAP=1` in netbase.config) only loads an existing import ("WRONG IMPORT" on fresh files)
  and never grows its files on macOS (file_allocate is a no-op there: SIGBUS on 0-byte maps);
- the binary puts its data in `<binary dir>/data/` whatever data_path says: run a copy (data/netbase/netbase);
- Docker (netbase.docker/) would need a Linux build of the C++ sources.
Next step for (b): the sysctl line once, then `./netbase :import cities` and `./netbase :server` from a copy in
data/netbase with a small netbase.config (SERVER_PORT=8181, maxNodes=2000000).
