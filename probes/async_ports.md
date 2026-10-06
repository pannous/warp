# Async and channel cases ported from other systems, in wasp's task syntax (wiki/async.md, notes/go_blocks.md): "code ||| expected"
# run: probes/function_calls.sh probes/async_ports.md (scratch/warp of this checkout)
# JS Promise.all / Python asyncio.gather: results in order
f(x) := { sleep(30); x * 10 }; await all [go f(1), go f(2), go f(3)] ||| [10 20 30]
jobs = []; for i in 1 to 3 { jobs.add(go { i * i }) }; sum(await all jobs) ||| 14
# Promise.race / asyncio.wait FIRST_COMPLETED: the first to finish
slow() := { sleep(300); 1 }; quick() := { sleep(10); 2 }; await any [go slow(), go quick()] ||| 2
slow() := { sleep(300); 1 }; quick() := { sleep(10); 2 }; await first [go slow(), go quick()] ||| 2
# Promise.allSettled: failures as values
bad() := [1]#5; good() := 3; try await go bad() else 0 ||| 0
# Kotlin async/await, coroutineScope: nested tasks
f(n) := { a = go { n + 1 }; b = go { n * 2 }; await a + await b }; await go f(5) ||| 16
# asyncio.wait_for / Kotlin withTimeout: a deadline
job = go { sleep(2000); 1 }; await job within 100 ms or 0 ||| 0
job = go { sleep(10); 7 }; await job within 1000 ms or 0 ||| 7
# cancellation: Kotlin job.cancel(), AbortController
job = go { sleep(3000); 1 }; stop job; 2 ||| 2
# Go: goroutines with a shared counter and WaitGroup
shared n = 0; jobs = []; for i in 1 to 4 { jobs.add(go { n += 1 }) }; await all jobs; n ||| 4
# Go channels: unbuffered handoff, range over a channel, select
ch = channel(); go { ch.send(42) }; ch.receive() ||| 42
ch = channel(); go { for i in 1 to 3 { ch.send(i) }; ch.close() }; total = 0; for v in ch { total += v }; total ||| 6
# JS async function / await of a value
f(x) := x + 1; await go f(1) ||| 2
# Python asyncio.sleep in tasks running concurrently
started = clock(); a = go { sleep(200); 1 }; b = go { sleep(200); 2 }; await a + await b; clock() - started < 350 ||| 1
