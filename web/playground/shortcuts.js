// The playground editor's keyboard shortcuts (card keyboard-shortcuts), all in SHORTCUTS: CodeMirror's key names,
// "Cmd-" for a Mac, "Ctrl-" for the others. A new shortcut is one more line there.

const LINE_COMMENT = "// ";
const COMMENTED = /^(\s*)\/\/ ?/;
const INDENT = /^\s*/;

// What toggles warp's line comments on `lines`: each edit {index, ch, removed, inserted}. All lines that are not blank
// commented: the markers go; else each such line gets one after the shallowest indentation (a lone blank line too)
function commentEdits(lines) {
	const filled = lines.map((line, index) => ({ line, index })).filter(({ line }) => line.trim());
	if (filled.length === 0) return lines.map((_, index) => ({ index, ch: 0, removed: 0, inserted: LINE_COMMENT }));
	if (filled.every(({ line }) => COMMENTED.test(line))) {
		return filled.map(({ line, index }) => {
			const [marker, indent] = line.match(COMMENTED);
			return { index, ch: indent.length, removed: marker.length - indent.length, inserted: "" };
		});
	}
	const ch = Math.min(...filled.map(({ line }) => line.match(INDENT)[0].length));
	return filled.map(({ index }) => ({ index, ch, removed: 0, inserted: LINE_COMMENT }));
}

// Cmd-/: the selected lines commented out or back in, else the cursor's line; a selection ending at the start of a
// line leaves that line alone. Edits in place keep the selection on the same text
function toggleComment(editor) {
	const from = editor.getCursor("from"), to = editor.getCursor("to");
	const last = to.line > from.line && to.ch === 0 ? to.line - 1 : to.line;
	const lines = Array.from({ length: last - from.line + 1 }, (_, offset) => editor.getLine(from.line + offset));
	editor.operation(() => {
		for (const { index, ch, removed, inserted } of commentEdits(lines)) {
			const line = from.line + index;
			editor.replaceRange(inserted, { line, ch }, { line, ch: ch + removed }, "+comment");
		}
	});
}

const SHORTCUTS = {
	"Ctrl-Enter": () => runPressed(),
	"Cmd-Enter": () => runPressed(),
	"Ctrl-/": toggleComment,
	"Cmd-/": toggleComment,
};
