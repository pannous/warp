// Keeps the character sequences a font shapes together in one face. fonts/fonts.css slices the large fonts by
// unicode-range and a browser shapes every slice on its own, which breaks IDS composition (⿰犭句), hieroglyph groups
// (𓀀𓐰𓁐) and TAG effects (你 + TAG M). watchSequences(element) wraps each such sequence in a span whose first font
// is a single unranged FontFace holding all of it (fonts/fonts.json, written by slice_fonts.py).
const FONTS = new URL("fonts/", import.meta.url);
const WRAPPED = "sequenceFont";

const inRanges = (point, ranges) => ranges.some(([first, last]) => point >= first && point <= last);
const faceNames = new Map();

function faceName(url) {
	if (!faceNames.has(url)) {
		const name = `uniscript sequence ${faceNames.size}`;
		document.fonts.add(new FontFace(name, `url("${url}")`));
		faceNames.set(url, name);
	}
	return faceNames.get(url);
}

function parseUnicodeRange(text) {
	return text.split(",").map(part => {
		const [first, last = first] = part.trim().replace(/^U\+/i, "").split("-");
		return [parseInt(first, 16), parseInt(last, 16)];
	});
}

/** the plain slices of the fonts whose slices carry TAG controls: [{ranges, url}] from the fonts.css rules */
function taggedSlices(tagged) {
	const sheet = [...document.styleSheets].find(sheet => sheet.href?.startsWith(FONTS.href));
	const rules = sheet ? [...sheet.cssRules].filter(rule => rule instanceof CSSFontFaceRule) : [];
	return rules
		.filter(rule => tagged.includes(rule.style.getPropertyValue("font-family").replace(/["']/g, "").trim()))
		.filter(rule => rule.style.getPropertyValue("unicode-range"))
		.map(rule => ({
			ranges: parseUnicodeRange(rule.style.getPropertyValue("unicode-range")),
			url: new URL(rule.style.getPropertyValue("src").match(/url\(["']?([^"')]+)/)[1], sheet.href).href,
		}));
}

const compareTuples = (left, right) => {
	for (let index = 0; index < Math.min(left.length, right.length); index++) if (left[index] !== right[index]) return left[index] - right[index];
	return left.length - right.length;
};

/** the face of the last bucket whose first operands are not after these operands */
function bucketUrl(buckets, operands) {
	let url = buckets[0][1];
	for (const [first, bucket] of buckets) if (compareTuples(operands, first) >= 0) url = bucket;
	return url;
}

/** [start, end, font url] of the sequences in a list of code points */
function sequences(points, { sequences: kinds, tag }, slices) {
	const found = [];
	const isTag = index => index < points.length && inRanges(points[index], tag);
	const afterTags = index => { while (isTag(index)) index++; return index; };
	const operators = new Map(kinds.filter(kind => kind.arity).map(kind => [kind.trigger[0][0], kind]));
	const aliases = new Map(kinds.find(kind => kind.aliases)?.aliases);
	// an IDS expression: an operator and its operands, which may be expressions themselves
	const expressionEnd = index => {
		const operator = operators.get(points[index]);
		let end = index + 1;
		for (let operand = 0; operator && operand < operator.arity && end < points.length; operand++) end = expressionEnd(end);
		return end;
	};
	for (let index = 0; index < points.length;) {
		const point = points[index];
		const run = kinds.find(kind => kind.run && inRanges(point, kind.run));
		if (run) {
			let end = index;
			while (end < points.length && inRanges(points[end], run.run)) end++;
			if (points.slice(index, end).some(point => inRanges(point, run.trigger))) found.push([index, end, run.url]);
			index = end;
			continue;
		}
		if (operators.has(point) && index + 1 < points.length) {
			const end = afterTags(expressionEnd(index));
			const operands = points.slice(index + 1, end).map(point => aliases.get(point) ?? point);
			found.push([index, end, bucketUrl(operators.get(point).buckets, operands)]);
			index = end;
			continue;
		}
		const slice = isTag(index + 1) && slices.find(slice => inRanges(point, slice.ranges));
		if (slice) {
			const end = afterTags(index + 1);
			found.push([index, end, slice.url]);
			index = end;
			continue;
		}
		index++;
	}
	return found.map(([start, end, url]) => [start, end, new URL(url, FONTS).href]);
}

function wrapTextNode(node, fonts, slices) {
	const points = [...node.data].map(character => character.codePointAt(0));
	const found = sequences(points, fonts, slices);
	if (!found.length) return;
	const inherited = getComputedStyle(node.parentElement).fontFamily;
	const text = (start, end) => String.fromCodePoint(...points.slice(start, end));
	const fragment = document.createDocumentFragment();
	let position = 0;
	for (const [start, end, url] of found) {
		fragment.append(text(position, start));
		const span = document.createElement("span");
		span.dataset[WRAPPED] = "";
		span.style.fontFamily = `"${faceName(url)}", ${inherited}`;
		span.textContent = text(start, end);
		fragment.append(span);
		position = end;
	}
	fragment.append(text(position, points.length));
	node.replaceWith(fragment);
}

function wrap(element, fonts, slices) {
	const walker = document.createTreeWalker(element, NodeFilter.SHOW_TEXT, {
		acceptNode: node => node.parentElement?.closest(`[data-sequence-font]`) ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT,
	});
	const nodes = [];
	while (walker.nextNode()) nodes.push(walker.currentNode);
	for (const node of nodes) wrapTextNode(node, fonts, slices);
}

/** Wraps the sequences in the elements now and whenever their content changes */
export async function watchSequences(...elements) {
	const fonts = await (await fetch(new URL("fonts.json", FONTS))).json();
	const slices = taggedSlices(fonts.tagged);
	for (const element of elements) {
		const observer = new MutationObserver(() => {
			observer.disconnect();
			wrap(element, fonts, slices);
			observer.observe(element, { childList: true, subtree: true, characterData: true });
		});
		wrap(element, fonts, slices);
		observer.observe(element, { childList: true, subtree: true, characterData: true });
	}
}
