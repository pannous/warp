// Exports seeded EZ Tree meshes for lib/meshlets.warp.
// EZ Tree by Daniel Greenheck, https://github.com/dgreenheck/ez-tree, MIT, Copyright (c) 2024 Daniel Greenheck.
// Pin: commit dcf309bd86bd521083d9c70f01f2de45fdc7c457. The npm package 1.1.0 has no createGeometry, so this uses
// that commit's source, bundled for Node with three left external:
//   git clone https://github.com/dgreenheck/ez-tree /tmp/ez-tree
//   git -C /tmp/ez-tree checkout dcf309bd86bd521083d9c70f01f2de45fdc7c457
//   (cd /tmp/ez-tree && npm ci --ignore-scripts)
//   (cd /tmp/ez-tree && npx esbuild src/lib/index.js --bundle --format=esm --platform=node --external:three --outfile=ez-tree.node.mjs)
//   node scripts/ez_tree/export.mjs /tmp/ez-tree/ez-tree.node.mjs samples/graphics/ez_trees
// No texture, model, font or sound is copied, and EZ Tree Pro is not used.
import { writeFileSync, mkdirSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const [bundle, out] = process.argv.slice(2);
if (!bundle || !out) throw new Error('usage: node export.mjs <ez-tree bundle .mjs> <output folder>');
const { Tree, TreePreset } = await import(pathToFileURL(resolve(bundle)).href);

// forest.warp species order: 0 pine, 1 oak, 2 birch (aspen stands in), 3 autumn (ash). Bush is the fifth asset.
// Leaf counts are the preset's except where the preset's own LOD still exceeds the triangle cap below: the skeleton,
// seed and branch shape stay the preset's. Fewer leaves, not a different tree.
const TREES = [
  { file: 'pine.eztree', preset: 'Pine Small', overrides: { leaves: { count: 6 } } },
  { file: 'oak.eztree', preset: 'Oak Small', overrides: { leaves: { count: 6 } } },
  { file: 'birch.eztree', preset: 'Aspen Small', overrides: {} },
  { file: 'autumn.eztree', preset: 'Ash Small', overrides: { leaves: { count: 8 } } },
  { file: 'bush.eztree', preset: 'Bush 2', overrides: { leaves: { count: 4 } } },
];
// Tree.defaultLODLevels[1] and [2]: the near mesh and the mid mesh.
const LEVELS = [
  { detail: { sectionStride: 3, segmentFactor: 0.75, leafStride: 2, leafScale: 1.25 }, most: 6000 },
  { detail: { sectionStride: 6, segmentFactor: 0.4, leafStride: 2, leafScale: 1.3, billboard: 'single' }, most: 2200 },
];
const BARK = 0;
const LEAF = 1;
const VERTEX_FLOATS = 5;
const FOLDER_CAP = 700000;

const merged = (geometries) => {
  const positions = [];
  const uvs = [];
  const triangles = [];
  for (const [geometry, part] of geometries) {
    const base = positions.length / 3;
    const position = geometry.getAttribute('position');
    const uv = geometry.getAttribute('uv');
    const index = geometry.getIndex();
    for (let i = 0; i < position.count; i++) {
      positions.push(position.getX(i), position.getY(i), position.getZ(i));
      uvs.push(uv.getX(i), uv.getY(i));
    }
    for (let i = 0; i < index.count; i += 3) {
      triangles.push([base + index.getX(i), base + index.getX(i + 1), base + index.getX(i + 2), part]);
    }
  }
  return { positions, uvs, triangles };
};

const meshOf = (tree, detail) => merged([[tree.createGeometry(detail).branches, BARK], [tree.createGeometry(detail).leaves, LEAF]]);

const point = (mesh, index) => [mesh.positions[3 * index], mesh.positions[3 * index + 1], mesh.positions[3 * index + 2]];
const sub = (a, b) => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const lengthOf = (a) => Math.hypot(a[0], a[1], a[2]);
const clamp01 = (x) => Math.min(1, Math.max(0, x));

// Distance from p to triangle abc (Ericson, Real-Time Collision Detection 5.1.5).
const toTriangle = (p, a, b, c) => {
  const ab = sub(b, a);
  const ac = sub(c, a);
  const ap = sub(p, a);
  const d1 = dot(ab, ap);
  const d2 = dot(ac, ap);
  if (d1 <= 0 && d2 <= 0) return lengthOf(ap);
  const bp = sub(p, b);
  const d3 = dot(ab, bp);
  const d4 = dot(ac, bp);
  if (d3 >= 0 && d4 <= d3) return lengthOf(bp);
  const vc = d1 * d4 - d3 * d2;
  if (vc <= 0 && d1 >= 0 && d3 <= 0) {
    const v = clamp01(d1 / (d1 - d3));
    return lengthOf(sub(ap, [ab[0] * v, ab[1] * v, ab[2] * v]));
  }
  const cp = sub(p, c);
  const d5 = dot(ab, cp);
  const d6 = dot(ac, cp);
  if (d6 >= 0 && d5 <= d6) return lengthOf(cp);
  const vb = d5 * d2 - d1 * d6;
  if (vb <= 0 && d2 >= 0 && d6 <= 0) {
    const w = clamp01(d2 / (d2 - d6));
    return lengthOf(sub(ap, [ac[0] * w, ac[1] * w, ac[2] * w]));
  }
  const va = d3 * d6 - d5 * d4;
  if (va <= 0 && d4 - d3 >= 0 && d5 - d6 >= 0) {
    const w = clamp01((d4 - d3) / (d4 - d3 + (d5 - d6)));
    const bc = sub(c, b);
    return lengthOf(sub(bp, [bc[0] * w, bc[1] * w, bc[2] * w]));
  }
  const denom = 1 / (va + vb + vc);
  const v = vb * denom;
  const w = vc * denom;
  return lengthOf(sub(ap, [ab[0] * v + ac[0] * w, ab[1] * v + ac[1] * w, ab[2] * v + ac[2] * w]));
};

// One-sided Hausdorff: how far the full tree's vertices are from a level's surface, in tree heights.
// Triangles live in a grid so each vertex only tests the cells that can still hold a closer one.
const errorOf = (full, level) => {
  const cell = 0.08;
  const key = (x, y, z) => (x + 512) * 1048576 + (y + 512) * 1024 + (z + 512);
  const grid = new Map();
  const boxes = [];
  level.triangles.forEach((triangle, t) => {
    const corners = [point(level, triangle[0]), point(level, triangle[1]), point(level, triangle[2])];
    const low = [Infinity, Infinity, Infinity];
    const high = [-Infinity, -Infinity, -Infinity];
    for (const corner of corners) {
      for (let k = 0; k < 3; k++) {
        low[k] = Math.min(low[k], corner[k]);
        high[k] = Math.max(high[k], corner[k]);
      }
    }
    boxes.push(corners);
    const ix0 = Math.floor(low[0] / cell);
    const iy0 = Math.floor(low[1] / cell);
    const iz0 = Math.floor(low[2] / cell);
    const ix1 = Math.floor(high[0] / cell);
    const iy1 = Math.floor(high[1] / cell);
    const iz1 = Math.floor(high[2] / cell);
    for (let ix = ix0; ix <= ix1; ix++) {
      for (let iy = iy0; iy <= iy1; iy++) {
        for (let iz = iz0; iz <= iz1; iz++) {
          const bucket = grid.get(key(ix, iy, iz));
          if (bucket) bucket.push(t);
          else grid.set(key(ix, iy, iz), [t]);
        }
      }
    }
  });
  const seen = new Uint32Array(level.triangles.length);
  let stamp = 1;
  let worst = 0;
  const count = full.positions.length / 3;
  for (let i = 0; i < count; i++) {
    const p = point(full, i);
    const ix = Math.floor(p[0] / cell);
    const iy = Math.floor(p[1] / cell);
    const iz = Math.floor(p[2] / cell);
    let best = Infinity;
    let radius = 0;
    while (radius < 64) {
      for (let dx = -radius; dx <= radius; dx++) {
        for (let dy = -radius; dy <= radius; dy++) {
          for (let dz = -radius; dz <= radius; dz++) {
            if (radius > 0 && Math.max(Math.abs(dx), Math.abs(dy), Math.abs(dz)) !== radius) continue;
            const bucket = grid.get(key(ix + dx, iy + dy, iz + dz));
            if (!bucket) continue;
            for (const t of bucket) {
              if (seen[t] === stamp) continue;
              seen[t] = stamp;
              const corners = boxes[t];
              const distance = toTriangle(p, corners[0], corners[1], corners[2]);
              if (distance < best) best = distance;
            }
          }
        }
      }
      if (radius >= 1 && radius * cell >= best) break;
      radius += 1;
      stamp += 1;
      if (stamp === 0xffffffff) {
        seen.fill(0);
        stamp = 1;
      }
    }
    if (best > worst) worst = best;
  }
  return worst;
};

// One tree height is 1. The trunk stays on x = 0, z = 0 (EZ Tree grows it from the origin) and the base sits at y = 0.
const normalized = (mesh) => {
  let low = Infinity;
  let high = -Infinity;
  for (let i = 1; i < mesh.positions.length; i += 3) {
    low = Math.min(low, mesh.positions[i]);
    high = Math.max(high, mesh.positions[i]);
  }
  const height = high - low;
  if (!(height > 0)) throw new Error('a tree has no height');
  const scale = 1 / height;
  const positions = mesh.positions.slice();
  for (let i = 0; i < positions.length; i += 3) {
    positions[i] *= scale;
    positions[i + 1] = (positions[i + 1] - low) * scale;
    positions[i + 2] *= scale;
  }
  return { ...mesh, positions };
};

const bytesOf = (levels) => {
  const size = 8 + levels.reduce((n, level) => n + 12 + 4 * VERTEX_FLOATS * (level.positions.length / 3) + 8 * level.triangles.length, 0);
  const view = new DataView(new ArrayBuffer(size));
  let at = 0;
  const u32 = (x) => { view.setUint32(at, x, true); at += 4; };
  const f32 = (x) => { view.setFloat32(at, x, true); at += 4; };
  const u16 = (x) => { view.setUint16(at, x, true); at += 2; };
  for (const ch of 'EZT1') view.setUint8(at++, ch.charCodeAt(0));
  u32(levels.length);
  for (const level of levels) {
    const vertices = level.positions.length / 3;
    u32(vertices);
    u32(level.triangles.length);
    f32(level.error);
    for (let i = 0; i < vertices; i++) {
      f32(level.positions[3 * i]);
      f32(level.positions[3 * i + 1]);
      f32(level.positions[3 * i + 2]);
      f32(level.uvs[2 * i]);
      f32(level.uvs[2 * i + 1]);
    }
    for (const triangle of level.triangles) for (const n of triangle) u16(n);
  }
  if (at !== size) throw new Error(`wrote ${at} of ${size}`);
  return new Uint8Array(view.buffer);
};

mkdirSync(out, { recursive: true });
const report = [];
let total = 0;
const canopyOnly = process.argv[4] === 'canopy';
for (const spec of (canopyOnly ? [] : TREES)) {
  const started = Date.now();
  const tree = new Tree();
  tree.options.copy(structuredClone(TreePreset[spec.preset]));
  tree.options.copy(spec.overrides);
  const full = normalized(meshOf(tree, {}));
  let previous = 0;
  const levels = LEVELS.map(({ detail, most }, index) => {
    const level = normalized(meshOf(tree, detail));
    const vertices = level.positions.length / 3;
    if (level.triangles.length > most) throw new Error(`${spec.preset}: ${level.triangles.length} triangles, more than ${most}`);
    if (vertices > 65535) throw new Error(`${spec.preset}: ${vertices} vertices, more than a u16 index holds`);
    // The finer stored mesh is the reference, so its error is 0, as a heightfield's fine grid is 0. The coarser
    // level's error is how far the full tree's vertices sit from that level, and it does not decrease.
    const measured = errorOf(full, level);
    level.measured = measured;
    level.error = index === 0 ? 0 : Math.max(measured, previous, 0.0001);
    previous = Math.max(previous, measured);
    return level;
  });
  const bytes = bytesOf(levels);
  writeFileSync(join(out, spec.file), bytes);
  total += bytes.length;
  const leaves = tree.options.leaves.count;
  report.push(`${spec.file}\t${spec.preset}\tseed ${tree.options.seed}\tleaves ${leaves}\tfull ${full.triangles.length}\t` +
    levels.map((level) => `${level.triangles.length} tris/${level.positions.length / 3} verts err ${level.error.toFixed(4)} measured ${level.measured.toFixed(4)}`).join('\t') +
    `\t${bytes.length} B\t${((Date.now() - started) / 1000).toFixed(1)} s`);
}
if (total > FOLDER_CAP) throw new Error(`${total} bytes in the folder, more than ${FOLDER_CAP}`);
console.log(report.join('\n'));
console.log(`total\t${total} B`);

// A second set for the million-tree horizon: the same presets and seeds, more of each preset's own
// leaves, and four details from EZ Tree's LOD strides (full or near-full, then defaultLODLevels[1]
// and [2], then a coarser stride). Geometry only, same as above. The two-level files stay, because
// the meshlet tests pin their triangle counts and the oak's coarse error.
const CANOPY = [
  {
    file: 'pine.eztree', preset: 'Pine Small', overrides: { leaves: { count: 8 } },
    levels: [
      { detail: { sectionStride: 2, segmentFactor: 0.9, leafStride: 1, leafScale: 1.1 }, most: 9000 },
      { detail: { sectionStride: 3, segmentFactor: 0.75, leafStride: 2, leafScale: 1.25 }, most: 6000 },
      { detail: { sectionStride: 6, segmentFactor: 0.4, leafStride: 2, leafScale: 1.3, billboard: 'single' }, most: 2500 },
      { detail: { sectionStride: 10, segmentFactor: 0.25, leafStride: 4, leafScale: 1.45, billboard: 'single' }, most: 1400 },
    ],
  },
  {
    file: 'oak.eztree', preset: 'Oak Small', overrides: { leaves: { count: 10 } },
    levels: [
      { detail: {}, most: 9000 },
      { detail: { sectionStride: 3, segmentFactor: 0.75, leafStride: 2, leafScale: 1.25 }, most: 6000 },
      { detail: { sectionStride: 6, segmentFactor: 0.4, leafStride: 2, leafScale: 1.3, billboard: 'single' }, most: 2500 },
      { detail: { sectionStride: 10, segmentFactor: 0.25, leafStride: 4, leafScale: 1.45, billboard: 'single' }, most: 1400 },
    ],
  },
  {
    file: 'birch.eztree', preset: 'Aspen Small', overrides: {},
    levels: [
      { detail: {}, most: 9000 },
      { detail: { sectionStride: 3, segmentFactor: 0.75, leafStride: 2, leafScale: 1.25 }, most: 6000 },
      { detail: { sectionStride: 6, segmentFactor: 0.4, leafStride: 2, leafScale: 1.3, billboard: 'single' }, most: 2500 },
      { detail: { sectionStride: 10, segmentFactor: 0.25, leafStride: 4, leafScale: 1.45, billboard: 'single' }, most: 1400 },
    ],
  },
  {
    file: 'autumn.eztree', preset: 'Ash Small', overrides: { leaves: { count: 12 } },
    levels: [
      { detail: {}, most: 9000 },
      { detail: { sectionStride: 3, segmentFactor: 0.75, leafStride: 2, leafScale: 1.25 }, most: 6000 },
      { detail: { sectionStride: 6, segmentFactor: 0.4, leafStride: 2, leafScale: 1.3, billboard: 'single' }, most: 2500 },
      { detail: { sectionStride: 10, segmentFactor: 0.25, leafStride: 4, leafScale: 1.45, billboard: 'single' }, most: 1400 },
    ],
  },
];
const CANOPY_CAP = 2500000;

const canopyLevels = (tree, specs) => {
  const full = normalized(meshOf(tree, {}));
  let previous = 0;
  return specs.map(({ detail, most }, index) => {
    const level = normalized(meshOf(tree, detail));
    const vertices = level.positions.length / 3;
    if (level.triangles.length > most) throw new Error(`${level.triangles.length} triangles, more than ${most}`);
    if (vertices > 65535) throw new Error(`${vertices} vertices, more than a u16 index holds`);
    const measured = errorOf(full, level);
    level.measured = measured;
    level.error = index === 0 ? 0 : Math.max(measured, previous, 0.0001);
    previous = Math.max(previous, measured);
    return level;
  });
};

if (process.argv[4] === 'canopy') {
  const canopy = join(out, 'canopy');
  mkdirSync(canopy, { recursive: true });
  let canopyTotal = 0;
  for (const spec of CANOPY) {
    const started = Date.now();
    const tree = new Tree();
    tree.options.copy(structuredClone(TreePreset[spec.preset]));
    tree.options.copy(spec.overrides);
    const levels = canopyLevels(tree, spec.levels);
    const bytes = bytesOf(levels);
    writeFileSync(join(canopy, spec.file), bytes);
    canopyTotal += bytes.length;
    console.log(`${spec.file}\tcanopy\tleaves ${tree.options.leaves.count}\t` +
      levels.map((level) => `${level.triangles.length} tris err ${level.error.toFixed(4)}`).join('\t') +
      `\t${bytes.length} B\t${((Date.now() - started) / 1000).toFixed(1)} s`);
  }
  if (canopyTotal > CANOPY_CAP) throw new Error(`${canopyTotal} bytes in canopy/, more than ${CANOPY_CAP}`);
  console.log(`canopy\t${canopyTotal} B`);
}
