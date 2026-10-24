#!/usr/bin/env python3
"""Moves line ranges of a Rust module into a child module (card cleanup-big-files), keeping every path and visibility:

    split_module.py <file.rs> "<child name>|<child doc line>|<first>-<last>[,<first>-<last>…]" […more children]

Several children at once, so the line numbers are all those of the file as it is. Ranges are 1-based and inclusive, cut at item boundaries (doc comments included). A range of methods (indented) goes
into the child wrapped in the header of the impl block it came from. The child starts with `use super::*`, which also
gives it the parent's private items; the parent gets `mod child; pub use child::*;`, so outside paths stay as they
were. Visibility is kept as it was: a private item becomes pub(super) (the parent's module), a pub(super) one names
the parent's parent, and `super::` paths in the moved code gain one level. Struct fields, macros and name clashes are
left to the compiler: fix what `cargo check` says by hand.
"""
import re
import sys
from pathlib import Path

ITEM = re.compile(r"^(\s*)(pub(?:\([^)]*\))? )?((?:const |unsafe |async |extern \"C\" )*(?:fn|const|static|struct|enum|type|trait|mod) )")


def module_path(file: Path) -> list[str]:
	parts = list(file.relative_to("src").with_suffix("").parts)
	if parts[-1] in ("mod", "lib", "main"):
		parts.pop()
	return ["crate"] + parts


def child_file(file: Path, child: str) -> Path:
	folder = file.parent if file.stem == "mod" else file.with_suffix("")
	return folder / f"{child}.rs"


def visible_in(path: list[str]) -> str:
	return "pub(crate) " if path == ["crate"] else f"pub(in {'::'.join(path)}) "


def opened_impl(lines: list[str], start: int) -> str:
	for line in reversed(lines[:start]):
		if line.startswith("impl"):
			return line
	raise SystemExit(f"no impl block above line {start + 1}")


def moved(lines: list[str], parent_parent: list[str], nested_parent: bool, in_trait=False) -> list[str]:
	"""in_trait: the lines are inside a trait or a trait impl, whose items take no visibility"""
	out = []
	in_struct = False
	for line in lines:
		if re.match(r"^(pub(\([^)]*\))? )?(unsafe )?(impl\b.* for |trait )", line):
			in_trait = True
		elif re.match(r"^(pub(\([^)]*\))? )?struct .*\{$", line):
			in_struct = True
		elif line.startswith("}"):
			in_trait = in_struct = False
		elif in_struct and re.match(r"^\t\w+:", line):
			line = "\tpub(super) " + line[1:]  # the parent reads and sets the fields of its child's structs
		item = ITEM.match(line)
		if item and len(item.group(1)) <= 1 and not (in_trait and item.group(1)):
			visibility = item.group(2)
			if visibility is None:
				visibility = "pub(super) "
			elif visibility == "pub(super) ":
				visibility = visible_in(parent_parent)
			line = item.group(1) + visibility + line[item.end(2) if item.group(2) else item.end(1):]
		if nested_parent:
			line = re.sub(r"(?<![\w:])super::", "super::super::", line)
		out.append(line)
	return out


def main():
	file, *children = sys.argv[1:]
	file = Path(file)
	lines = file.read_text().split("\n")
	parent = module_path(file)
	nested_parent = file.stem != "mod"  # a child of x.rs lives one level below x's siblings; of mod.rs, beside them
	parent_parent = parent[:-1] if len(parent) > 1 else parent
	all_spans, declarations, imports = [], [], set()
	for spec in children:
		child, doc_and_ranges = spec.split("|", 1)
		doc, ranges = doc_and_ranges.rsplit("|", 1)
		spans = [tuple(int(n) - 1 for n in span.split("-")) for span in ranges.split(",")]
		body = []
		for first, last in spans:
			part = lines[first:last + 1]
			imports.update(line for line in part if line.startswith("use "))  # imports stay with the parent, which the child sees
			part = [line for line in part if not line.startswith("use ")]
			if part[0].startswith(("\t", " ")):
				opened = opened_impl(lines, first)
				body += [opened] + moved(part, parent_parent, nested_parent, " for " in opened) + ["}", ""]
			else:
				body += moved(part, parent_parent, nested_parent) + [""]
		target = child_file(file, child)
		target.parent.mkdir(exist_ok=True)
		target.write_text("\n".join([f"//! {doc}", "use super::*;", ""] + body).rstrip("\n") + "\n")
		print(f"{target}: {len(body)} lines")
		all_spans += spans
		declarations.append(f"mod {child};")
		visibilities = {item.group(2) for item in map(ITEM.match, body) if item and not item.group(1)}
		if visibilities:  # methods alone need no re-export; the re-export is as visible as the most visible item
			wider = sorted(v for v in visibilities if v.startswith("pub(in "))
			reexport = next((v for v in ["pub ", "pub(crate) "] + wider if v in visibilities), "")
			declarations.append(f"{reexport}use {child}::*;")
	for first, last in sorted(all_spans, reverse=True):
		del lines[first:last + 1]
	at = next(
		index
		for index, line in enumerate(lines)
		if line.startswith("///") or line and not line.startswith(("use ", "//", "#!", "\t", " ", "}", "pub use ", "pub(crate) use "))
	)
	lines[at:at] = sorted(imports) + declarations + [""]
	file.write_text("\n".join(lines))


main()
