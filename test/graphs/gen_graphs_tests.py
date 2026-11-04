#!/usr/bin/env python3
"""Generate test/modules/test_graphs.pairs.warp. Expected values come from small python reference implementations
that follow lib/graphs.warp's order rules (insertion order); each one is cross-checked against networkx (sets, lengths,
costs, validity) when networkx is importable. Rerun from the repository root: python3 test/graphs/gen_graphs_tests.py
(networkx: python3 -m venv /tmp/nxv && /tmp/nxv/bin/pip install networkx && /tmp/nxv/bin/python test/graphs/...)"""
import os, random, heapq
from collections import deque
try:
    import networkx as nx
except ImportError:
    nx = None
root = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..")
lit = lambda s: "'" + s.replace("\\", "\\\\").replace("'", "\\'") + "'"
def W(x):
    if isinstance(x, bool): return "yes" if x else "no"
    if isinstance(x, str): return '"' + x.replace("\\", "\\\\").replace('"', '\\"') + '"'
    if isinstance(x, (list, tuple)): return "[" + ", ".join(W(y) for y in x) + "]"
    return str(x)
L = ["// lib/graphs.warp: traversals, paths, orders, components and spanning trees; expected values from",
     "// test/graphs/gen_graphs_tests.py (python references in insertion order, cross-checked with networkx), card lib-graphs"]
def case(code, exp): L.append(lit("use graphs; " + code) + "\t" + exp)

class G:
    def __init__(s, edges, directed):
        s.directed, s.nodes, s.adj = directed, [], {}
        for e in edges:
            u, v, w = e[0], e[1], (e[2] if len(e) > 2 else 1)
            for n in (u, v):
                if n not in s.adj: s.nodes.append(n); s.adj[n] = []
            s.adj[u].append((v, w))
            if not directed and u != v: s.adj[v].append((u, w))
        s.edges = [tuple(e) + ((1,) if len(e) == 2 else ()) for e in edges]
        s.ix = {n: i for i, n in enumerate(s.nodes)}
    def nx(s):
        g = (nx.DiGraph if s.directed else nx.Graph)()
        g.add_nodes_from(s.nodes)
        for u, v, w in s.edges: g.add_edge(u, v, weight=w)
        return g
def bfs(g, s):
    seen, out, q = {s}, [], deque([s])
    while q:
        u = q.popleft(); out.append(u)
        for v, _ in g.adj[u]:
            if v not in seen: seen.add(v); q.append(v)
    return out
def dfs(g, s):
    seen, out, st = set(), [], [s]
    while st:
        u = st.pop()
        if u in seen: continue
        seen.add(u); out.append(u)
        for v, _ in reversed(g.adj[u]):
            if v not in seen: st.append(v)
    return out
def sp(g, a, b):
    prev, q, seen = {a: None}, deque([a]), {a}
    while q:
        u = q.popleft()
        if u == b:
            p = []
            while u is not None: p.append(u); u = prev[u]
            return p[::-1]
        for v, _ in g.adj[u]:
            if v not in seen: seen.add(v); prev[v] = u; q.append(v)
    return []
def dij(g, s):
    dist, done, h = {s: 0}, set(), [(0, s)]
    while h:
        d, u = heapq.heappop(h)
        if u in done: continue
        done.add(u)
        for v, w in g.adj[u]:
            if v not in done and (v not in dist or d + w < dist[v]): dist[v] = d + w; heapq.heappush(h, (d + w, v))
    return [[n, dist[n]] for n in g.nodes if n in dist]
def topo(g):
    indeg = {n: 0 for n in g.nodes}
    for n in g.nodes:
        for v, _ in g.adj[n]: indeg[v] += 1
    ready = [g.ix[n] for n in g.nodes if indeg[n] == 0]; heapq.heapify(ready); out = []
    while ready:
        u = g.nodes[heapq.heappop(ready)]; out.append(u)
        for v, _ in g.adj[u]:
            indeg[v] -= 1
            if indeg[v] == 0: heapq.heappush(ready, g.ix[v])
    return out
def cc(g):
    both = {n: [v for v, _ in g.adj[n]] for n in g.nodes}
    if g.directed:
        for n in g.nodes:
            for v, _ in g.adj[n]: both[v].append(n)
    seen, out = set(), []
    for s in g.nodes:
        if s in seen: continue
        seen.add(s); part, q = [], deque([s])
        while q:
            u = q.popleft(); part.append(u)
            for v in both[u]:
                if v not in seen: seen.add(v); q.append(v)
        out.append(part)
    return out
def scc(g):  # recursive Tarjan (small graphs only), the same completion order as the iterative one
    num, low, on, st, out, c = {}, {}, set(), [], [], [0]
    def go(u):
        c[0] += 1; num[u] = low[u] = c[0]; st.append(u); on.add(u)
        for v, _ in g.adj[u]:
            if v not in num: go(v); low[u] = min(low[u], low[v])
            elif v in on: low[u] = min(low[u], num[v])
        if low[u] == num[u]:
            m = set()
            while True:
                k = st.pop(); on.discard(k); m.add(k)
                if k == u: break
            out.append([n for n in g.nodes if n in m])
    for n in g.nodes:
        if n not in num: go(n)
    return out
def mst(g):
    parent = {n: n for n in g.nodes}
    def root(x):
        while parent[x] != x: parent[x] = parent[parent[x]]; x = parent[x]
        return x
    out = []
    for e in sorted(g.edges, key=lambda e: e[2]):
        a, b = root(e[0]), root(e[1])
        if a != b: parent[b] = a; out.append(list(e))
    return out
def valid_topo(g, order):
    pos = {n: i for i, n in enumerate(order)}
    return len(order) == len(g.nodes) and all(pos[u] < pos[v] for u, v, _ in g.edges)
def check(g):
    if not nx: return
    n = g.nx()
    for s in g.nodes:
        assert set(bfs(g, s)) == set(nx.descendants(n, s)) | {s}
        assert dfs(g, s) == list(nx.dfs_preorder_nodes(n, s))
        assert dict((a, b) for a, b in dij(g, s)) == nx.single_source_dijkstra_path_length(n, s)
    if g.directed:
        assert sorted(map(sorted, scc(g))) == sorted(sorted(c) for c in nx.strongly_connected_components(n))
        assert sorted(map(sorted, cc(g))) == sorted(sorted(c) for c in nx.weakly_connected_components(n))
        if nx.is_directed_acyclic_graph(n): assert valid_topo(g, topo(g))
    else:
        assert sorted(map(sorted, cc(g))) == sorted(sorted(c) for c in nx.connected_components(n))
        assert sum(e[2] for e in mst(g)) == nx.minimum_spanning_tree(n).size(weight="weight")
def make(edges, directed): return "graph_of(%s, %s)" % (W(edges), "yes" if directed else "no")

L.append("// building: edge lists ([u, v] or [u, v, w]) and maps; nodes and neighbours in insertion order")
e1 = [["a", "b", 4], ["a", "c", 1], ["c", "b", 2], ["b", "d", 1], ["c", "d", 5], ["e", "f", 1]]
g1 = G(e1, True); check(g1)
case("graph_nodes(%s)" % make(e1, True), W(g1.nodes))
case("graph_neighbors(%s, \"a\")" % make(e1, True), W([v for v, _ in g1.adj["a"]]))
case("graph_neighbors(%s, \"b\")" % make(e1, False), W([v for v, _ in G(e1, False).adj["b"]]))
case("graph_nodes(graph_of({\"x\": [\"y\", [\"z\", 3]], \"w\": []}, yes))", W(["x", "y", "z", "w"]))
case("graph_neighbors(graph_of({\"x\": [\"y\", [\"z\", 3]]}, yes), \"x\")", W(["y", "z"]))
case("graph_node_count(graph_of([[1, 2], [2, 3], [1, 3]], no))", "3")
case("graph_nodes(graph_of([[1, \"1\"]], yes))", W([1, "1"]))
case("try { bfs(%s, \"zz\") } catch e { e.message }" % make(e1, True), W("graphs: no node zz"))

L.append("// bfs, dfs, has_path and shortest_path, unreachable nodes included")
for directed in (True, False):
    g = G(e1, directed)
    for s in ["a", "d", "e"]:
        case("bfs(%s, %s)" % (make(e1, directed), W(s)), W(bfs(g, s)))
        case("dfs(%s, %s)" % (make(e1, directed), W(s)), W(dfs(g, s)))
    for a, b in [("a", "d"), ("d", "a"), ("a", "f"), ("a", "a")]:
        case("shortest_path(%s, %s, %s)" % (make(e1, directed), W(a), W(b)), W(sp(g, a, b)))
        case("has_path(%s, %s, %s)" % (make(e1, directed), W(a), W(b)), W(b in bfs(g, a)))
case("has_path(%s, \"a\", \"nowhere\")" % make(e1, True), "no")

L.append("// dijkstra, dijkstra_path and a_star; ties, unreachable targets and negative weights")
case("dijkstra(%s, \"a\")" % make(e1, True), W(dij(g1, "a")))
case("dijkstra(%s, \"d\")" % make(e1, False), W(dij(G(e1, False), "d")))
case("dijkstra_path(%s, \"a\", \"d\")" % make(e1, True), W([["a", "c", "b", "d"], 4]))
case("dijkstra_path(%s, \"a\", \"f\")" % make(e1, True), "[[], ø]")
tie = [["s", "x", 1], ["s", "y", 1], ["x", "t", 1], ["y", "t", 1]]
if nx: assert nx.dijkstra_path_length(G(tie, True).nx(), "s", "t") == 2
case("dijkstra_path(%s, \"s\", \"t\")" % make(tie, True), W([["s", "x", "t"], 2]))
case("shortest_path(%s, \"s\", \"t\")" % make(tie, True), W(["s", "x", "t"]))
case("try { dijkstra(graph_of([[1, 2, 3], [2, 3, -1]], yes), 1) } catch e { e.message }", W("dijkstra: negative weight -1 on 2 -> 3"))
case("try { a_star(graph_of([[1, 2, -2]], yes), 1, 2, n => 0) } catch e { e.message }", W("a_star: negative weight -2 on 1 -> 2"))
case("dijkstra(graph_of([[1, 2, 1/2], [2, 3, 1/3]], yes), 1)", "[[1, 0], [2, 1/2], [3, 5/6]]")
# a grid with the manhattan heuristic, checked against networkx's astar_path_length
grid = []
for r in range(4):
    for c in range(4):
        if c < 3: grid.append([r * 4 + c, r * 4 + c + 1, 1 + (r + c) % 3])
        if r < 3: grid.append([r * 4 + c, r * 4 + c + 4, 1 + (r * c) % 2])
gg = G(grid, False); check(gg)
best = dict((a, b) for a, b in dij(gg, 0))[15]
if nx: assert nx.astar_path_length(gg.nx(), 0, 15, heuristic=lambda a, b: abs(a % 4 - b % 4) + abs(a // 4 - b // 4)) == best
case("(a_star(%s, 0, 15, n => (3 - n %% 4) + (3 - n / 4 as int)))#2" % make(grid, False), W(best))
case("(dijkstra_path(%s, 0, 15))#2" % make(grid, False), W(best))
case("count((a_star(%s, 0, 15, n => 0))#1) >= 7" % make(grid, False), "yes")

L.append("// topological_sort: insertion order among ready nodes, a cycle reported with its nodes")
dag = [["shirt", "tie"], ["tie", "jacket"], ["pants", "shoes"], ["pants", "belt"], ["belt", "jacket"], ["shirt", "belt"], ["socks", "shoes"]]
gd = G(dag, True); check(gd)
assert valid_topo(gd, topo(gd))
case("topological_sort(%s)" % make(dag, True), W(topo(gd)))
case("try { topological_sort(graph_of([[\"a\", \"b\"], [\"b\", \"c\"], [\"c\", \"a\"], [\"x\", \"a\"]], yes)) } catch e { e.message }", W("topological_sort: cycle b -> c -> a -> b"))
case("try { topological_sort(graph_of([[1, 1]], yes)) } catch e { e.message }", W("topological_sort: cycle 1 -> 1"))
case("try { topological_sort(graph_of([[1, 2]], no)) } catch e { e.message }", W("topological_sort: the graph is undirected"))

L.append("// connected_components (weak for directed graphs) and strongly_connected_components (iterative Tarjan)")
sc = [[1, 2], [2, 3], [3, 1], [3, 4], [4, 5], [5, 6], [6, 4], [7, 6], [7, 8], [8, 7], [9, 9]]
gs = G(sc, True); check(gs)
case("strongly_connected_components(%s)" % make(sc, True), W(scc(gs)))
case("connected_components(%s)" % make(sc, True), W(cc(gs)))
case("connected_components(%s)" % make(e1, False), W(cc(G(e1, False))))
case("strongly_connected_components(%s)" % make(e1, True), W(scc(g1)))

L.append("// kruskal_mst: union-find, weight ties broken by edge order; a forest for disconnected graphs")
mt = [["a", "b", 7], ["a", "d", 5], ["b", "c", 8], ["b", "d", 9], ["b", "e", 7], ["c", "e", 5], ["d", "e", 15], ["d", "f", 6], ["e", "f", 8], ["e", "g", 9], ["f", "g", 11], ["x", "y", 1]]
gm = G(mt, False); check(gm)
case("kruskal_mst(%s)" % make(mt, False), W(mst(gm)))
case("mst_weight(kruskal_mst(%s))" % make(mt, False), W(sum(e[2] for e in mst(gm))))
case("kruskal_mst(graph_of([[1, 2, 1], [2, 3, 1], [1, 3, 1]], no))", W([[1, 2, 1], [2, 3, 1]]))
case("try { kruskal_mst(graph_of([[1, 2]], yes)) } catch e { e.message }", W("kruskal_mst: the graph is directed"))

L.append("// random graphs, every algorithm against the references (and networkx)")
rnd = random.Random(7)
for k in range(6):
    n, directed = rnd.randint(5, 9), k % 2 == 0
    es = [[rnd.randrange(n), rnd.randrange(n), rnd.randint(0, 5)] for _ in range(rnd.randint(n, 2 * n))]
    seen, uniq = set(), []
    for e in es:
        key = (e[0], e[1]) if directed else frozenset((e[0], e[1]))
        if key not in seen: seen.add(key); uniq.append(e)
    es = uniq  # one edge a pair: networkx's simple graphs keep only one weight
    g = G(es, directed); check(g); s = g.nodes[0]
    case("bfs(%s, %s)" % (make(es, directed), W(s)), W(bfs(g, s)))
    case("dfs(%s, %s)" % (make(es, directed), W(s)), W(dfs(g, s)))
    case("dijkstra(%s, %s)" % (make(es, directed), W(s)), W(dij(g, s)))
    case("connected_components(%s)" % make(es, directed), W(cc(g)))
    if directed: case("strongly_connected_components(%s)" % make(es, directed), W(scc(g)))
    else: case("mst_weight(kruskal_mst(%s))" % make(es, directed), W(sum(e[2] for e in mst(g))))

L.append("// a 10k-node chain: everything iterative, no recursion depth")
chain = "graph_of([[i, i + 1] for i in 1 to 9999], yes)"
case("count(dfs(%s, 1))" % chain, "10000")
case("(bfs(%s, 1))#10000" % chain, "10000")
case("count(shortest_path(%s, 1, 10000))" % chain, "10000")
case("(dijkstra_path(%s, 1, 10000))#2" % chain, "9999")
case("(topological_sort(%s))#10000" % chain, "10000")
case("count(strongly_connected_components(%s))" % chain, "10000")
case("has_path(%s, 1, 10000)" % chain, "yes")
case("mst_weight(kruskal_mst(graph_of([[i, i + 1, i % 7] for i in 1 to 9999], no)))", W(sum(i % 7 for i in range(1, 10000))))
open(os.path.join(root, "test", "modules", "test_graphs.pairs.warp"), "w").write("\n".join(L) + "\n")
print(len([l for l in L if not l.startswith("//")]), "cases", "(networkx %s)" % nx.__version__ if nx else "(no networkx)")
