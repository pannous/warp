#!/usr/bin/env python3
"""Bake USDCraft-10K objects into the rigid bodies samples/physics/usd_collisions.warp drops.

usage: scripts/usdcraft-bodies.py <object dir>... > samples/physics/usdcraft.bodies
  an object dir holds meta.json and physx/model.usdc (nvidia/USDCraft-10K, Apache-2.0); needs usd-core and numpy:
  python3 -m venv v && v/bin/pip install usd-core numpy

Each object becomes one rigid body: its links welded in their rest pose (the joints are listed, not simulated).
- mass: the sum of the links' authored physics:mass (kg). The stages author no inertia or centre of mass (PhysX derives
  them from the colliders), so both come from the collision prims here: every Cube, Sphere and Mesh (its bounding box
  in its own frame, whatever physics:approximation says: convexHull, convexDecomposition, sdf) is a solid of its
  link's mass shared by volume.
- frame: the principal axes through the centre of mass, so the inertia is diagonal; `pose` turns that frame back into
  the stage's (Z up, metres).
- collision proxy: spheres of one radius on a grid that fills the object's bounds, kept where the sphere's centre is
  within a collision prim grown by half the grid's spacing, and not buried under six kept neighbours (the surface shell
  is what touches).
Output lines (lengths in m, mass in kg, inertia in kg m², angles in degrees):
  body <name> <object_id> <title words…>
  mass m · inertia ixx iyy izz · com x y z · pose qw qx qy qz · friction μ · restitution e · color r g b (sRGB 0…1)
  sphere x y z r          (in the body frame)
  joint <type> <name> <axis> <lower> <upper>
  end
"""
import json, math, sys
from pathlib import Path
import numpy as np
from pxr import Usd, UsdGeom, UsdPhysics, UsdShade

CELLS = 6  # grid cells along the object's longest side
MAX_SPHERES = 48


def prim_solid(prim, cache):
    """(kind, half extents or radius, centre in its frame, frame→stage 4x4) of one collision prim, or None"""
    m = np.array(cache.GetLocalToWorldTransform(prim))  # row vectors: p_world = p @ m[:3,:3] + m[3,:3]
    t = prim.GetTypeName()
    if t == "Cube":
        s = UsdGeom.Cube(prim).GetSizeAttr().Get() or 2.0
        return "box", np.array([s / 2] * 3), np.zeros(3), m
    if t == "Sphere":
        r = UsdGeom.Sphere(prim).GetRadiusAttr().Get() or 1.0
        return "ball", np.array([r] * 3), np.zeros(3), m
    if t in ("Cylinder", "Capsule"):
        g = UsdGeom.Cylinder(prim) if t == "Cylinder" else UsdGeom.Capsule(prim)
        r, h = g.GetRadiusAttr().Get(), g.GetHeightAttr().Get()
        half = {"X": [h / 2, r, r], "Y": [r, h / 2, r], "Z": [r, r, h / 2]}[g.GetAxisAttr().Get() or "Z"]
        return "box", np.array(half), np.zeros(3), m
    if t == "Mesh":
        pts = np.array(UsdGeom.Mesh(prim).GetPointsAttr().Get() or [], dtype=float)
        if len(pts) == 0:
            return None
        lo, hi = pts.min(0), pts.max(0)
        return "box", (hi - lo) / 2, (hi + lo) / 2, m
    return None


def scaled_frame(m):
    """the frame's rotation and per-axis scale (row-vector convention)"""
    a = m[:3, :3]
    scale = np.linalg.norm(a, axis=1)
    return a / scale[:, None], scale


def solid_inertia(kind, half, mass):
    if kind == "ball":
        return np.eye(3) * 0.4 * mass * half[0] ** 2
    x, y, z = 2 * half
    return np.diag([y * y + z * z, x * x + z * z, x * x + y * y]) * mass / 12


def look_color(stage, link):
    """the diffuse colour of the material most of a link's visual faces are bound to"""
    for prim in Usd.PrimRange(link):
        if prim.GetTypeName() == "Mesh" and "/Visuals/" in str(prim.GetPath()):
            mat, _ = UsdShade.MaterialBindingAPI(prim).ComputeBoundMaterial()
            if mat:
                for s in Usd.PrimRange(mat.GetPrim()):
                    if s.GetTypeName() == "Shader":
                        c = UsdShade.Shader(s).GetInput("diffuseColor")
                        if c and c.Get() is not None:
                            return [float(v) for v in c.Get()]
    return [0.6, 0.6, 0.6]


def quat_of(r):
    """unit quaternion (w, x, y, z) of a rotation matrix acting on column vectors"""
    w = math.sqrt(max(0.0, 1 + r[0, 0] + r[1, 1] + r[2, 2])) / 2
    x = math.copysign(math.sqrt(max(0.0, 1 + r[0, 0] - r[1, 1] - r[2, 2])) / 2, r[2, 1] - r[1, 2])
    y = math.copysign(math.sqrt(max(0.0, 1 - r[0, 0] + r[1, 1] - r[2, 2])) / 2, r[0, 2] - r[2, 0])
    z = math.copysign(math.sqrt(max(0.0, 1 - r[0, 0] - r[1, 1] + r[2, 2])) / 2, r[1, 0] - r[0, 1])
    n = math.sqrt(w * w + x * x + y * y + z * z)
    return [w / n, x / n, y / n, z / n]


def body(folder):
    folder = Path(folder)
    meta = json.loads((folder / "meta.json").read_text())
    stage = Usd.Stage.Open(str(folder / "physx" / "model.usdc"))
    assert UsdGeom.GetStageUpAxis(stage) == "Z" and UsdGeom.GetStageMetersPerUnit(stage) == 1.0
    cache = UsdGeom.XformCache()
    solids, colors, friction, restitution = [], [], [], []
    for link in stage.Traverse():
        if not link.HasAPI(UsdPhysics.RigidBodyAPI):
            continue
        mass = UsdPhysics.MassAPI(link).GetMassAttr().Get() or 0.0
        parts = [s for s in (prim_solid(p, cache) for p in Usd.PrimRange(link) if p.HasAPI(UsdPhysics.CollisionAPI)) if s]
        vols = []
        for kind, half, centre, m in parts:
            _, scale = scaled_frame(m)
            h = half * scale
            vols.append(4 / 3 * math.pi * h[0] ** 3 if kind == "ball" else 8 * h.prod())
        total = sum(vols) or 1.0
        for (kind, half, centre, m), v in zip(parts, vols):
            solids.append((kind, half, centre, m, mass * v / total))
        colors.append((mass, look_color(stage, link)))
    for p in stage.Traverse():
        if p.HasAPI(UsdPhysics.MaterialAPI):
            mat = UsdPhysics.MaterialAPI(p)
            friction.append(mat.GetDynamicFrictionAttr().Get())
            restitution.append(mat.GetRestitutionAttr().Get())
    mass = sum(s[4] for s in solids)
    # centre of mass and inertia in the stage frame
    com = sum(s[4] * (s[2] @ s[3][:3, :3] + s[3][3, :3]) for s in solids) / mass
    inertia = np.zeros((3, 3))
    for kind, half, centre, m, ms in solids:
        rot, scale = scaled_frame(m)
        local = solid_inertia(kind, half * scale, ms)
        r = rot.T  # column-vector rotation of the prim's frame into the stage's
        d = centre @ m[:3, :3] + m[3, :3] - com
        inertia += r @ local @ r.T + ms * (d @ d * np.eye(3) - np.outer(d, d))
    principal, axes = np.linalg.eigh(inertia)
    # the principal axis nearest each stage axis, pointing its way: an upright object keeps pose ≈ 1 0 0 0
    order = []
    for k in range(3):
        order.append(max((i for i in range(3) if i not in order), key=lambda i: abs(axes[k, i])))
    principal, axes = principal[order], axes[:, order]
    axes *= np.sign(np.diag(axes) + 1e-12)
    if np.linalg.det(axes) < 0:
        axes[:, 2] *= -1
    # sphere proxy: a grid over the object's bounds, in the stage frame
    corners = []
    for kind, half, centre, m, _ in solids:
        for sx in (-1, 1):
            for sy in (-1, 1):
                for sz in (-1, 1):
                    corners.append((centre + half * [sx, sy, sz]) @ m[:3, :3] + m[3, :3])
    corners = np.array(corners)
    lo, hi = corners.min(0), corners.max(0)
    cells = CELLS
    size = hi - lo
    while True:
        # one radius; on each axis spheres from lo + r to hi - r, at most 1.8 r apart, so the shell's outside is the
        # objects's bounds and no gap between neighbours lets a sphere through
        radius = min(size.max() / (2 * cells), size.min() / 2)
        n = np.array([max(1, int(math.ceil((e - 2 * radius) / (1.8 * radius) - 1e-9)) + 1) for e in size])
        gap = np.array([(e - 2 * radius) / (k - 1) if k > 1 else 0.0 for e, k in zip(size, n)])
        start = np.array([lo[a] + radius if n[a] > 1 else (lo[a] + hi[a]) / 2 for a in range(3)])
        full = np.zeros(n, bool)
        for idx in np.ndindex(*n):
            p = start + np.array(idx) * gap
            for kind, half, centre, m, _ in solids:
                # the sphere's centre, in the prim's frame, within the prim grown by half the spacing (a radius at most)
                rot, scale = scaled_frame(m)
                q = ((p - m[3, :3]) @ rot.T) / scale - centre
                grow = np.maximum(gap / 2, 1e-6).min() / scale
                if kind == "ball":
                    if np.linalg.norm(q) <= half[0] + grow.min():
                        full[idx] = True
                        break
                elif np.all(np.abs(q) <= half + grow):
                    full[idx] = True
                    break
        shell = []
        for idx in np.ndindex(*n):
            if not full[idx]:
                continue
            nb = [tuple(np.add(idx, d)) for d in ((1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1))]
            buried = all(all(0 <= a < b for a, b in zip(k, n)) and full[k] for k in nb)
            if not buried:
                shell.append(start + np.array(idx) * gap)
        if len(shell) <= MAX_SPHERES or cells <= 2:
            break
        cells -= 1
    spheres = [((p - com) @ axes, radius) for p in shell]
    # the heaviest link's colour, linear → sRGB for the screen
    color = [12.92 * c if c <= 0.0031308 else 1.055 * c ** (1 / 2.4) - 0.055 for c in max(colors)[1]]
    name = folder.name.split("_", 1)[1]
    out = [f"body {name} {meta['object_id']} {meta['title']}",
           f"mass {mass:.6g}",
           "inertia " + " ".join(f"{v:.6g}" for v in principal),
           "com " + " ".join(f"{v:.5f}" for v in com),
           "pose " + " ".join(f"{v:.6f}" for v in quat_of(axes)),
           f"friction {min(friction):.3g}",
           f"restitution {max(restitution):.3g}",
           "color " + " ".join(f"{v:.3f}" for v in color)]
    out += ["sphere " + " ".join(f"{v:.5f}" for v in p) + f" {r:.5f}" for p, r in spheres]
    for j in stage.Traverse():
        if j.IsA(UsdPhysics.Joint) and j.GetTypeName() != "PhysicsFixedJoint":
            kind = j.GetTypeName().replace("Physics", "").replace("Joint", "").lower()
            get = lambda a: j.GetAttribute(a).Get() if j.HasAttribute(a) else None
            out.append(f"joint {kind} {j.GetName()} {get('physics:axis')} {get('physics:lowerLimit')} {get('physics:upperLimit')}")
    out.append("end")
    return out, meta


def main(folders):
    print("# rigid bodies baked by scripts/usdcraft-bodies.py from nvidia/USDCraft-10K (Apache-2.0, see usdcraft.NOTICE)")
    for f in folders:
        lines, meta = body(f)
        print(f"# {meta['object_id']} {meta['physx_sha256'] if 'physx_sha256' in meta else meta['assets']['physx']['sha256']}")
        print("\n".join(lines))


if __name__ == "__main__":
    main(sys.argv[1:])
