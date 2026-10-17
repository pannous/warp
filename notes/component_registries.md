# WebAssembly component registries

## The ecosystem
- Components and WIT packages are distributed as OCI artifacts in ordinary container registries, mostly ghcr.io.
  warg and wa.dev (the Bytecode Alliance's own registry protocol) were archived in 2025; the wasm-pkg docs list only
  the registry types "oci" and "local" now.
- Client: Bytecode Alliance wasm-pkg-tools, CLI `wkg` (~0.16): `wkg get wasi:http@0.2.1`, `wkg oci pull <ref>`,
  `wkg publish x.wasm --package ns:name@ver`. Rust crates: wasm-pkg-client, wasm-pkg-common.
- Name → OCI reference: `ns:name@ver` → `<registry>/<prefix><ns>/<name>:<ver>`, e.g. wasi:http@0.2.1 →
  ghcr.io/webassembly/wasi/http:0.2.1.
- Which registry: `https://<namespace domain>/.well-known/wasm-pkg/registry.json`, e.g. wasi.dev →
  `{ociRegistry: "ghcr.io", ociNamespacePrefix: "webassembly/"}`, bytecodealliance.org → prefix
  `bytecodealliance/wasm-pkg/`. Built-in defaults: wasi → wasi.dev, ba → bytecodealliance.org; anything else is mapped
  per namespace in `~/.config/wasm-pkg/config.toml`.
- WASI publishes every interface package there with each release (ghcr.io/webassembly/wasi/{http,cli,io,…}).

## What an artifact holds (anonymous reads work)
- token: `GET https://ghcr.io/token?scope=repository:<repo>:pull` → `{token}`, then Bearer on every request.
- tags: `GET /v2/<repo>/tags/list` (skip `sha256-….sig` signature tags).
- manifest (`Accept: application/vnd.oci.image.manifest.v1+json`): annotations
  `org.opencontainers.image.{description,version,source,licenses,url}`, config media type
  `application/vnd.wasm.config.v0+json` (that marks a Wasm artifact; Docker images differ), one `application/wasm` layer.
- config blob: `{architecture: "wasm", os: "wasip2", component: {exports: [...], imports: [...]}}`. A WIT-only package
  exports its own interfaces and imports nothing; a runnable component imports wasi:* interfaces.

## No search
OCI has no public catalog: ghcr.io refuses `/v2/_catalog`, and GitHub's REST package listing needs the read:packages
scope. Our own index fills the gap: scripts/component_index.py crawls
- the public HTML package pages of known owners (`https://github.com/orgs/<owner>/packages?page=N`, `?tab=packages` for
  users), anonymous,
- owners found by GitHub code search for publishing commands (`wkg oci push`, `wkg publish`, the wasm config media type),
keeps only Wasm artifacts and writes components.warp (repository root, next to packages.warp): a short name per
package → package name, OCI reference, newest version, description, exports/imports.

## Short names
warp doesn't adopt the long `ns:name` names: `use component http` looks the short name up in components.warp, the way
config.toml maps namespaces to registries. A short name is the package name (`http`); on a clash wasi wins and the
others get `<ns>-<name>`. `use component wasi:http@0.2.1` and full OCI references stay possible for anything unlisted.
