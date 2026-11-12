// One full-detail level of an EZ Tree medium preset, plus the geometry's own normals.
// Same EZT1 layout as export.mjs. Does not rewrite the two-level files the meshlet tests pin.
//   node scripts/ez_tree/hero.mjs /tmp/ez-tree/ez-tree.node.mjs samples/graphics/ez_trees/hero
import { writeFileSync, mkdirSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const [bundle, out] = process.argv.slice(2);
if (!bundle || !out) throw new Error('usage: node hero.mjs <ez-tree bundle .mjs> <output folder>');
const { Tree, TreePreset, Billboard } = await import(pathToFileURL(resolve(bundle)).href);

const HEROES = ['Ash Medium', 'Oak Medium', 'Pine Medium', 'Aspen Medium'];
// Pine Medium and Aspen Medium are sparse in the published preset (30 small needle cards,
// 11 aspen cards). Same seed and skeleton; more and larger cards, still under a u16 index.
// Oak and Ash Medium stay the preset the demo opens on. `tuned` writes only the two we change.
const FULLER = {
  'Pine Medium': { count: 64, size: 2.4 },
  'Aspen Medium': { count: 40, size: 3.6 },
};
const BARK = 0;
const LEAF = 1;

const merged = (geometry) => {
  const positions = [];
  const normals = [];
  const uvs = [];
  const triangles = [];
  for (const [part, mesh] of [['bark', geometry.branches], ['leaf', geometry.leaves]]) {
    const base = positions.length / 3;
    const position = mesh.getAttribute('position');
    const normal = mesh.getAttribute('normal');
    const uv = mesh.getAttribute('uv');
    const index = mesh.getIndex();
    const partId = part === 'bark' ? BARK : LEAF;
    for (let i = 0; i < position.count; i++) {
      positions.push(position.getX(i), position.getY(i), position.getZ(i));
      normals.push(normal.getX(i), normal.getY(i), normal.getZ(i));
      uvs.push(uv.getX(i), uv.getY(i));
    }
    for (let i = 0; i < index.count; i += 3) {
      triangles.push([base + index.getX(i), base + index.getX(i + 1), base + index.getX(i + 2), partId]);
    }
  }
  return { positions, normals, uvs, triangles };
};

const normalized = (mesh) => {
  let low = Infinity;
  let high = -Infinity;
  for (let i = 1; i < mesh.positions.length; i += 3) {
    low = Math.min(low, mesh.positions[i]);
    high = Math.max(high, mesh.positions[i]);
  }
  const height = high - low;
  const scale = 1 / height;
  const positions = mesh.positions.slice();
  for (let i = 0; i < positions.length; i += 3) {
    positions[i] *= scale;
    positions[i + 1] = (positions[i + 1] - low) * scale;
    positions[i + 2] *= scale;
  }
  return { ...mesh, positions, height, low };
};

const bytesOf = (level) => {
  const vertices = level.positions.length / 3;
  const size = 8 + 12 + 20 * vertices + 8 * level.triangles.length;
  const view = new DataView(new ArrayBuffer(size));
  let at = 0;
  const u32 = (x) => { view.setUint32(at, x, true); at += 4; };
  const f32 = (x) => { view.setFloat32(at, x, true); at += 4; };
  const u16 = (x) => { view.setUint16(at, x, true); at += 2; };
  for (const ch of 'EZT1') view.setUint8(at++, ch.charCodeAt(0));
  u32(1);
  u32(vertices);
  u32(level.triangles.length);
  f32(0);
  for (let i = 0; i < vertices; i++) {
    f32(level.positions[3 * i]);
    f32(level.positions[3 * i + 1]);
    f32(level.positions[3 * i + 2]);
    f32(level.uvs[2 * i]);
    f32(level.uvs[2 * i + 1]);
  }
  for (const triangle of level.triangles) for (const n of triangle) u16(n);
  if (at !== size) throw new Error(`wrote ${at} of ${size}`);
  return new Uint8Array(view.buffer);
};

const normalBytes = (level) => {
  const view = new DataView(new ArrayBuffer(level.normals.length * 4));
  for (let i = 0; i < level.normals.length; i++) view.setFloat32(i * 4, level.normals[i], true);
  return new Uint8Array(view.buffer);
};

// The second level is the same tree with a wider stride, for front trees past the
// first few that still have to be meshes. It is not a different preset.
const COARSE = { sectionStride: 3, segmentFactor: 0.7, leafStride: 3, leafScale: 1.3 };
// Same skeleton, one card in eight. Close crowns stay on the coarser file above;
// this one is what still fits once those have taken the triangle cap.
const FAR = { sectionStride: 8, segmentFactor: 0.4, leafStride: 8, leafScale: 1.8, billboard: Billboard.Single };

mkdirSync(out, { recursive: true });
for (const preset of HEROES) {
  const tree = new Tree();
  tree.options.copy(structuredClone(TreePreset[preset]));
  const fuller = FULLER[preset];
  if (fuller) {
    tree.options.leaves.count = fuller.count;
    tree.options.leaves.size = fuller.size;
  }
  const tuned = process.argv[4] === 'tuned';
  const file = preset.toLowerCase().replace(' ', '_') + '.eztree';
  if (fuller || !tuned) {
    const level = normalized(merged(tree.createGeometry({})));
    const vertices = level.positions.length / 3;
    if (vertices > 65535) throw new Error(`${preset}: ${vertices} vertices`);
    writeFileSync(join(out, file), bytesOf(level));
    writeFileSync(join(out, file.replace('.eztree', '.nrm')), normalBytes(level));
    console.log([preset, 'full', `${level.triangles.length} tris`, `${vertices} verts`, `height ${level.height.toFixed(4)}`].join('\t'));
  }
  const coarse = normalized(merged(tree.createGeometry(COARSE)));
  const coarseVerts = coarse.positions.length / 3;
  if (coarseVerts > 65535) throw new Error(`${preset} coarse: ${coarseVerts} vertices`);
  writeFileSync(join(out, file.replace('.eztree', '_coarse.eztree')), bytesOf(coarse));
  console.log([preset, 'coarse', `${coarse.triangles.length} tris`, `${coarseVerts} verts`].join('\t'));
  const far = normalized(merged(tree.createGeometry(FAR)));
  const farVerts = far.positions.length / 3;
  if (farVerts > 65535) throw new Error(`${preset} far: ${farVerts} vertices`);
  writeFileSync(join(out, file.replace('.eztree', '_far.eztree')), bytesOf(far));
  console.log([preset, 'far', `${far.triangles.length} tris`, `${farVerts} verts`].join('\t'));
}
