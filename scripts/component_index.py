#!/usr/bin/env python3
"""Index of WebAssembly components and WIT packages published as OCI artifacts (notes/component_registries.md).

  scripts/component_index.py            crawl and write components.warp
  scripts/component_index.py search X   list index entries whose name, package, description or interfaces contain X

OCI registries have no search, so the crawl starts from known owners plus owners whose code publishes with wkg,
reads their public GitHub package pages and keeps every artifact whose config is a Wasm config.
"""
import concurrent.futures
import json
import pathlib
import re
import subprocess
import sys
import urllib.error
import urllib.parse
import urllib.request

INDEX = pathlib.Path(__file__).resolve().parent.parent / "components.warp"
REGISTRY = "ghcr.io"
# earlier owners win when two publish the same package (webassembly hosts wasi.dev's official WASI packages)
SEED_OWNERS = ["webassembly", "bytecodealliance", "componentized", "spinframework", "wasmcloud", "microsoft"]
DISCOVERY_QUERIES = ["wkg oci push", "wkg publish", "application/vnd.wasm.config.v0+json", "wash push"]
DISCOVERY_LIMIT = 100
WASM_CONFIG = "application/vnd.wasm.config.v0+json"
MANIFEST_TYPES = "application/vnd.oci.image.manifest.v1+json, application/vnd.docker.distribution.manifest.v2+json"
MAX_PAGES = 30
WORKERS = 16
TIMEOUT = 30
PREFERRED_NAMESPACE = "wasi"
CORE_MODULE_SECTION, COMPONENT_SECTION = 1, 4
PACKAGE_LINK = re.compile(r'/packages/container/package/([^"?#]+)')
STABLE_VERSION = re.compile(r"^v?(\d+)\.(\d+)\.(\d+)$")
ANY_VERSION = re.compile(r"^v?(\d+)\.(\d+)\.(\d+)")


def fetch(url, headers=None):
    request = urllib.request.Request(url, headers={"User-Agent": "warp-component-index", **(headers or {})})
    with urllib.request.urlopen(request, timeout=TIMEOUT) as response:
        return response.read()


def fetch_json(url, headers=None):
    return json.loads(fetch(url, headers))


def discovered_owners():
    owners = []
    for query in DISCOVERY_QUERIES:
        found = subprocess.run(["gh", "search", "code", query, "--limit", str(DISCOVERY_LIMIT), "--json", "repository",
                                "-q", ".[].repository.nameWithOwner"], capture_output=True, text=True)
        if found.returncode:
            print(f"code search '{query}' failed: {found.stderr.strip()}", file=sys.stderr)
        owners += [line.split("/")[0] for line in found.stdout.split()]
    return owners


def package_page_urls(owner):
    yield lambda page: f"https://github.com/orgs/{owner}/packages?page={page}"
    yield lambda page: f"https://github.com/{owner}?tab=packages&page={page}"


def owner_packages(owner):
    """The container package paths an owner lists publicly, e.g. 'wasi/http' for webassembly."""
    for page_url in package_page_urls(owner):
        packages = []
        for page in range(1, MAX_PAGES + 1):
            try:
                html = fetch(page_url(page)).decode("utf-8", "replace")
            except urllib.error.HTTPError:
                break
            found = [urllib.parse.unquote(path) for path in PACKAGE_LINK.findall(html)]
            new = [path for path in dict.fromkeys(found) if path not in packages]
            if not new:
                break
            packages += new
        if packages:
            return packages
    return []


def version_key(tag):
    match = ANY_VERSION.match(tag)
    return (bool(STABLE_VERSION.match(tag)), *map(int, match.groups())) if match else None


def newest_tag(tags):
    versions = [tag for tag in tags if version_key(tag)]
    if versions:
        return max(versions, key=version_key)
    return "latest" if "latest" in tags else None


def leb128(data, position):
    value = shift = 0
    while True:
        byte = data[position]
        position += 1
        value |= (byte & 0x7F) << shift
        shift += 7
        if byte < 0x80:
            return value, position


def has_code(wasm):
    """Whether a component binary embeds a core module or component, unlike an encoded WIT package (types only)."""
    position = 8
    while position < len(wasm):
        section = wasm[position]
        size, position = leb128(wasm, position + 1)
        if section in (CORE_MODULE_SECTION, COMPONENT_SECTION):
            return True
        position += size
    return False


def inspect(owner, path):
    """The index entry of one OCI repository, or None when it is no Wasm artifact."""
    repository = f"{owner}/{path}".lower()
    try:
        token = fetch_json(f"https://{REGISTRY}/token?scope=repository:{repository}:pull")["token"]
        auth = {"Authorization": f"Bearer {token}"}
        base = f"https://{REGISTRY}/v2/{repository}"
        tag = newest_tag(fetch_json(f"{base}/tags/list", auth).get("tags") or [])
        if not tag:
            return None
        manifest = fetch_json(f"{base}/manifests/{tag}", {**auth, "Accept": MANIFEST_TYPES})
        config = manifest.get("config", {})
        if config.get("mediaType") != WASM_CONFIG:
            return None
        component = fetch_json(f"{base}/blobs/{config['digest']}", auth).get("component") or {}
        exports, imports = component.get("exports") or [], component.get("imports") or []
        runnable = bool(imports) or has_code(fetch(f"{base}/blobs/{manifest['layers'][0]['digest']}", auth))
    except (urllib.error.URLError, TimeoutError, KeyError, IndexError, ValueError) as error:
        print(f"skip {repository}: {error}", file=sys.stderr)
        return None
    segments = path.split("/")
    namespace, name = segments[-2:] if len(segments) > 1 else (owner, segments[0])
    name = name.removesuffix(".wasm")
    annotations = manifest.get("annotations") or {}
    return {
        "package": f"{namespace}:{name}".lower(),
        "oci": f"{REGISTRY}/{repository}",
        "version": tag,
        "kind": "component" if runnable else "interface",
        "description": annotations.get("org.opencontainers.image.description", ""),
        "repository": annotations.get("org.opencontainers.image.source", ""),
        "exports": exports,
        "imports": imports,
    }


def crawl():
    owners = list(dict.fromkeys(SEED_OWNERS + discovered_owners()))
    print(f"{len(owners)} owners", file=sys.stderr)
    with concurrent.futures.ThreadPoolExecutor(WORKERS) as pool:
        listed = list(pool.map(owner_packages, owners))
        candidates = [(owner, path) for owner, paths in zip(owners, listed) for path in paths]
        print(f"{len(candidates)} container packages", file=sys.stderr)
        entries = [entry for entry in pool.map(lambda candidate: inspect(*candidate), candidates) if entry]
    return entries


def first_per_package(entries):
    """One entry per package name: the earliest owner in crawl order (seeds first) wins."""
    chosen = {}
    for entry in entries:
        chosen.setdefault(entry["package"], entry)
    return list(chosen.values())


def short_names(entries):
    """name → entry: the bare package name, `<namespace>-<name>` for those that lose a clash."""
    by_name = {}
    for entry in entries:
        by_name.setdefault(entry["package"].split(":")[1], []).append(entry)
    named = {}
    for name, clashing in by_name.items():
        clashing.sort(key=lambda entry: entry["package"].split(":")[0] != PREFERRED_NAMESPACE)
        named[name] = clashing[0]
        for entry in clashing[1:]:
            named[entry["package"].replace(":", "-")] = entry
    return dict(sorted(named.items()))


def warp_value(value):
    if isinstance(value, list):
        return "[" + ", ".join(map(warp_value, value)) + "]"
    return json.dumps(value, ensure_ascii=False)


def write_index(named):
    lines = [
        "// WebAssembly components and WIT packages on OCI registries: `use component <name>` (card use-component-registry).",
        "// Generated by scripts/component_index.py, don't edit; notes/component_registries.md.",
        "// kind interface: a WIT package without code; component: runnable, imports what it needs.",
    ]
    for name, entry in named.items():
        fields = ", ".join(f"{key}: {warp_value(value)}" for key, value in entry.items() if value not in ("", []))
        lines.append(f"{warp_value(name)}: {{{fields}}}")
    INDEX.write_text("\n".join(lines) + "\n")
    print(f"{len(named)} entries → {INDEX}", file=sys.stderr)


def read_index():
    entries = {}
    for line in INDEX.read_text().splitlines():
        if line.startswith('"'):
            name, fields = line.split(": ", 1)
            entries[json.loads(name)] = fields
    return entries


def search(term):
    term = term.lower()
    for name, fields in read_index().items():
        if term in name.lower() or term in fields.lower():
            print(f"{name}: {fields}")


if __name__ == "__main__":
    if sys.argv[1:2] == ["search"]:
        search(" ".join(sys.argv[2:]))
    else:
        write_index(short_names(first_per_package(crawl())))
