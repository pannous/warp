#!/usr/bin/env python3
"""Turns the release tarballs into PyPI wheels of warp-language (`pip install warp-language`), one per platform.

A wheel holds no Python: the binaries go in warp_language-<version>.data/scripts/, which pip and uv install into the
environment's bin directory, so warp-runtime lands next to warp (the layout maturin's bin bindings produce).
Usage: build_wheels.py <version> <dir with warp-v<version>-<target>.tar.gz> <output dir>; notes/packaging.md
"""

import base64
import hashlib
import io
import re
import stat
import sys
import tarfile
import zipfile
from pathlib import Path

# warp-lang on PyPI is NVIDIA's Warp (simulation framework); deb, rpm and the AUR keep warp-lang
DISTRIBUTION = "warp_language"
PACKAGE_NAME = "warp-language"
SUMMARY = "Warp: a data notation that is also a programming language, compiled to WebAssembly GC"
HOMEPAGE = "https://warp.pannous.com/"
REPOSITORY = "https://github.com/pannous/warp"
LICENSE_EXPRESSION = "AGPL-3.0-or-later AND MIT"  # the compiler AGPL, warp-runtime MIT (CONTRIBUTING.md)
# macOS minimum versions are Rust's defaults per architecture (otool -l: minos); Linux reads glibc from the binary
MACOS_PLATFORMS = {
    "aarch64-apple-darwin": "macosx_11_0_arm64",
    "x86_64-apple-darwin": "macosx_10_12_x86_64",
}
LINUX_ARCHITECTURES = {
    "x86_64-unknown-linux-gnu": "x86_64",
    "aarch64-unknown-linux-gnu": "aarch64",
}
EXECUTABLE_MODE = (stat.S_IFREG | 0o755) << 16  # pip only marks regular files executable


def newest_glibc(binary: bytes) -> tuple[int, int]:
    versions = {(int(major), int(minor)) for major, minor in re.findall(rb"GLIBC_(\d+)\.(\d+)", binary)}
    return max(versions)


def platform_tag(target: str, binaries: dict[str, bytes]) -> str:
    if target in MACOS_PLATFORMS:
        return MACOS_PLATFORMS[target]
    major, minor = max(newest_glibc(binary) for binary in binaries.values())
    return f"manylinux_{major}_{minor}_{LINUX_ARCHITECTURES[target]}"


def read_binaries(archive: Path) -> dict[str, bytes]:
    with tarfile.open(archive) as tar:
        return {member.name: tar.extractfile(member).read() for member in tar.getmembers() if member.isfile()}


def metadata(version: str, readme: str) -> str:
    return (
        "Metadata-Version: 2.4\n"
        f"Name: {PACKAGE_NAME}\n"
        f"Version: {version}\n"
        f"Summary: {SUMMARY}\n"
        f"License-Expression: {LICENSE_EXPRESSION}\n"
        f"Project-URL: Homepage, {HOMEPAGE}\n"
        f"Project-URL: Repository, {REPOSITORY}\n"
        "Classifier: Programming Language :: Rust\n"
        "Classifier: Topic :: Software Development :: Compilers\n"
        "Description-Content-Type: text/markdown\n"
        f"\n{readme}"
    )


def record_line(path: str, content: bytes) -> str:
    digest = base64.urlsafe_b64encode(hashlib.sha256(content).digest()).rstrip(b"=").decode()
    return f"{path},sha256={digest},{len(content)}"


def write_wheel(version: str, target: str, binaries: dict[str, bytes], readme: str, output: Path) -> Path:
    tag = f"py3-none-{platform_tag(target, binaries)}"
    dist_info = f"{DISTRIBUTION}-{version}.dist-info"
    entries = [(f"{DISTRIBUTION}-{version}.data/scripts/{name}", content, True) for name, content in binaries.items()]
    entries += [
        (f"{dist_info}/METADATA", metadata(version, readme).encode(), False),
        (f"{dist_info}/WHEEL", f"Wheel-Version: 1.0\nGenerator: warp build_wheels.py\nRoot-Is-Purelib: false\nTag: {tag}\n".encode(), False),
    ]
    record_path = f"{dist_info}/RECORD"
    record = "\n".join([record_line(path, content) for path, content, _ in entries] + [f"{record_path},,", ""])
    entries.append((record_path, record.encode(), False))

    wheel = output / f"{DISTRIBUTION}-{version}-{tag}.whl"
    with zipfile.ZipFile(wheel, "w", zipfile.ZIP_DEFLATED) as archive:
        for path, content, executable in entries:
            info = zipfile.ZipInfo(path, date_time=(2020, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = EXECUTABLE_MODE if executable else (stat.S_IFREG | 0o644) << 16
            archive.writestr(info, content)
    return wheel


def main(version: str, tarballs: str, output: str) -> None:
    readme = (Path(__file__).resolve().parents[2] / "README.md").read_text()
    Path(output).mkdir(parents=True, exist_ok=True)
    archives = sorted(Path(tarballs).glob(f"warp-v{version}-*.tar.gz"))
    if not archives:
        sys.exit(f"no warp-v{version}-*.tar.gz in {tarballs}")
    for archive in archives:
        target = archive.name.removeprefix(f"warp-v{version}-").removesuffix(".tar.gz")
        print(write_wheel(version, target, read_binaries(archive), readme, Path(output)))


if __name__ == "__main__":
    main(*sys.argv[1:])
