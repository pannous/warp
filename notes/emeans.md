# e-means (card implement-new)

The user asked for "the new e-means algorithm". That is the name of a real, recent paper, not a mishearing:
Bessa, Mazaré, Szilvasy, Touvron, Picard, Douze, Jégou (FAIR), "e-means: outperforming k-means (in one-tenth the
epochs)", arXiv 2611.09501, 6 Nov 2026, https://arxiv.org/abs/2611.09501, https://e-means.github.io (code in Faiss).
No other recent algorithm by that name fits; K*-means (arXiv 2505.11904) is a different, parameter-free one.

## What it is
Mini-batch k-means whose centroids are exponential moving averages (the VQ-VAE codebook rule), made into a k-means
solver (Algorithm 1):
- per cluster a count N_j and a sum S_j; the centroid is S_j / N_j, never stored. Start: k random points, N_j = 1e-10
- each batch of B points from a fresh shuffle per epoch: assign to the nearest centroid, then
  N_j ← γ N_j + (1-γ) n_j, S_j ← γ S_j + (1-γ) s_j
- the step α = 1-γ is annealed on a cosine from α0 down to 0.1 α0 over the M = T⌊n/B⌋ updates
- every 5 batches, a cluster with N_j < 0.1 × mean count is dead: it takes half of a donor cluster drawn ∝ N (each
  donor once), the donor's centroid nudged by ×(1±δ, 1∓δ, …) both ways, δ = 2^-7, each half keeping N/2
- finalization: the mean of the points each cluster got in the last epoch (better train error); a slow EMA is the
  alternative for out-of-sample error (not built)
- scaling laws fix B and α0 from (k, n, T), so nothing is tuned: α0 = min(0.824 k^-0.46 B^0.90 n^-0.45 T^-0.11, 1),
  B = 0.035 k^0.13 n^0.88 T^0.81 rounded to a power of 2, at most n (÷8 above n = 2M, not built)

Against classic k-means (Lloyd): Lloyd moves every centroid once per pass over all n points; e-means moves them after
every small batch, smoothed and with a shrinking step, so it reaches Lloyd's 25-epoch loss in 1.4 to 2.6 epochs on
embedding datasets (10-45× GPU speed-ups), and dead-cluster revival keeps every centroid in use.

## In warp
lib/clustering.warp (`use clustering`): kmeans, emeans (a Clustering: centroids, labels, error), and kmeans_steps /
emeans_steps, the centroids after each step for animations. Test: test/clustering. Sample: samples/clustering/emeans.warp,
the two side by side for the same 3 epochs.

On 600 2-D points in 8 blobs, k = 8, 3 epochs each, e-means had the lower error in 20 of 20 seeds (Lloyd after 3
passes is far from converged). Against Lloyd's 25 iterations it lost more often than it won: the paper itself says
its laws are tuned for high-dimensional data and are not optimal on tiny, low-dimensional sets (its Figure 11).

Found on the way: random's shuffle and sample reordered the caller's list (fixed); cards
default-parameter-constant, comprehension-variable-scope, frame-variable-shadowing, test-modules (seed 42).
