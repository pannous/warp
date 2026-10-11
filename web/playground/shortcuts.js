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

// The selected lines, else the cursor's line; a selection ending at the start of a line leaves that line alone
function selectedLines(editor) {
	const first = editor.getCursor("from").line, to = editor.getCursor("to");
	return { first, last: to.line > first && to.ch === 0 ? to.line - 1 : to.line };
}

const linesBetween = (editor, first, last) => Array.from({ length: last - first + 1 }, (_, offset) => editor.getLine(first + offset));

// Cmd-/: the selected lines commented out or back in. Edits in place keep the selection on the same text
function toggleComment(editor) {
	const { first, last } = selectedLines(editor);
	const lines = linesBetween(editor, first, last);
	editor.operation(() => {
		for (const { index, ch, removed, inserted } of commentEdits(lines)) {
			const line = first + index;
			editor.replaceRange(inserted, { line, ch }, { line, ch: ch + removed }, "+comment");
		}
	});
}

// The moved lines and their neighbour above (step -1) or below (step 1), after the move: the neighbour on the other side
const movedPast = (lines, step) => step < 0 ? [...lines.slice(1), lines[0]] : [lines.at(-1), ...lines.slice(0, -1)];

// Cmd-Ctrl-Up/Down: the selected lines move one line up or down, the selection with them; at the edge nothing moves
function moveLines(editor, step) {
	const { first, last } = selectedLines(editor);
	const top = Math.min(first, first + step), bottom = Math.max(last, last + step);
	if (top < 0 || bottom >= editor.lineCount()) return;
	const [anchor, head] = ["anchor", "head"].map(end => editor.getCursor(end));
	const moved = movedPast(linesBetween(editor, top, bottom), step).join("\n");
	editor.operation(() => {
		editor.replaceRange(moved, { line: top, ch: 0 }, { line: bottom, ch: editor.getLine(bottom).length }, "+move");
		editor.setSelection({ line: anchor.line + step, ch: anchor.ch }, { line: head.line + step, ch: head.ch });
	});
}

const SHORTCUTS = {
	"Ctrl-Enter": () => runPressed(),
	"Cmd-Enter": () => runPressed(),
	"Ctrl-/": toggleComment,
	"Cmd-/": toggleComment,
	"Cmd-Ctrl-Up": editor => moveLines(editor, -1),
	"Cmd-Ctrl-Down": editor => moveLines(editor, 1),
	"Alt-Up": editor => moveLines(editor, -1),
	"Alt-Down": editor => moveLines(editor, 1),
};
