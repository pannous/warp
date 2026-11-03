# Sharing data between tasks

Each `go` task works in its own room (its own WebAssembly instance, notes/threads.md). Two tasks never hold the same
tree, list or object; they pass copies or notes. What to do depends on what the tasks do with the data.

## The tasks only read it: pass it in
Each task gets its own copy.

```warp
tree = load_tree()
a = go count_leaves(tree)      # gets a copy
b = go find_max(tree)          # gets a copy too
await a + await b
```

The only cost is the copy. For a huge tree and many tasks, give each task only the part it needs (`tree.left`,
`tree.right`).

## Each task builds a piece: return it, assemble at the end
The common case: nobody shares, everyone delivers a result.

```warp
left  = go build(part1)
right = go build(part2)
tree  = {left: await left, right: await right}
```

## Several tasks change the same tree: one owner, the others send changes
Only the owner touches the tree; the others send their changes through a channel and the owner applies them one by one.
One writer, so nothing gets garbled (the actor pattern; a sketch, the channel words may be spelled differently):

```warp
changes = channel()
owner = go {
    tree = {}
    for change in changes { tree = apply(tree, change) }
    tree
}
go { changes.send(add "x") }
go { changes.send(add "y") }
changes.close()
tree = await owner
```

## Only numbers are truly shared: `shared`
Counters and sums: `shared` makes atomic int/float cells and arrays every task reaches.

```warp
shared hits = 0
go { hits += 1 }       # atomic, no update lost
```

## Why `shared tree` is refused
`shared tree = …` is an error: "a tree can't be shared between tasks; pass it (it is copied) or send results through a
channel". A tree several tasks change at once would need locks, the source of the hard threading bugs (races,
deadlocks). warp leaves them out on purpose.

## How the copy works
Passing or returning a value copies it deeply into the other instance: the task changes its own copy, and the result
comes back as a copy. An object referenced twice in the tree stays one object in the copy, and cycles work. What
cannot cross at all (an open file, a window) is an error that names it.
