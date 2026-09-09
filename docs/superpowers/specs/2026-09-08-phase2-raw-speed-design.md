# Phase 2: Raw speed

Second of three phases. Goal: more samples per second on both backends with
no change to what the renderers compute, except where noted.

## Measurement

* `benches/render.rs` (criterion): CPU `render_linear` on a fixed 200-sphere
  scene at a small resolution. Run before and after.
* GPU: wall-clock of `examples/cornell --gpu` and `examples/one-weekend --gpu`
  before and after.

## CPU

1. **Iterative path loop.** `radiance` becomes a loop carrying throughput
   instead of recursing. Same emission, Russian roulette and depth semantics.
2. **Sphere test early-out.** Half-b quadratic, no square root unless the
   discriminant is positive.
3. **Flat BVH.** Nodes live in one `Vec` in depth-first order (left child at
   `index + 1`, right child index stored), leaves index into the shape list.
   Split on the longest axis at the median, the same rule the GPU builder
   uses. Public `BVH::from_vec(Vec<Hitable>)` API is unchanged.
4. **Inverse ray direction computed once per ray** at the top of BVH
   traversal and reused by every slab test.
5. **Threading stays on scoped threads** pulling tiles from an atomic
   counter (added in phase 1). That already gives dynamic load balancing, so
   rayon is not added.

## GPU

1. **Traversal stays test-on-pop, with the inverse ray direction computed
   once per ray.** Two orderings were measured on `one-weekend --gpu` and
   rejected: testing both children at the parent (28.9 s versus 16.7 s, it
   doubles node loads) and ordering children by the ray direction sign along
   the stored split axis (17.3 s versus 16.7 s, no gain). The CPU showed the
   same pattern (0.52 s versus 0.69 s single-threaded), so both backends use
   the simple traversal.
2. **Sphere packed to 32 bytes**: centre and radius in one `vec4`, material
   index plus padding in a second.
3. **Cosine-weighted Lambertian sampling** on both backends, replacing
   `normal + random_in_unit_sphere()`. Two random numbers and no rejection
   loop. This is the correct Lambertian distribution; images change very
   slightly and the parity test guards that both backends change together.

## Realtime viewer

1. Blit texture and both bind groups created once (and on resize), not per
   frame.
2. Accumulation reset via `clear_buffer` instead of uploading a zeroed
   vector.
3. Samples per frame adapt to the measured frame time, targeting about 16 ms,
   so the view stays responsive while moving and converges faster when still.
4. Intermediate blit texture is `Rgba16Float` to avoid 8-bit quantisation of
   linear light before the sRGB surface.

## Tests

* BVH versus brute force: many random rays against a random sphere field must
  return the same nearest hit.
* Existing depth, disc UV and parity tests must stay green.

## Results

Measured on an Apple M4 Pro. CPU numbers are single-threaded on the bench
scene; GPU numbers are the example programs' own timings. The cosine sampler
makes paths bounce more before escaping, so equal-time comparisons understate
the per-bounce speedup; the "old sampler" column isolates the code changes.

| Workload                    | Before  | After (old sampler) | After   |
|-----------------------------|---------|---------------------|---------|
| CPU bench, 1 thread         | 1.43 M paths/s | n/a          | 2.37 M paths/s |
| `one-weekend --gpu`         | 18.2 s  | 16.7 s              | 19.2 s  |
| `cornell --gpu`             | 30.9 s  | n/a                 | 27.5 s  |

Realtime viewer: per-frame allocations removed, accumulation cleared on the
GPU, samples per frame adapt to measured tracing time (one sample per frame
at 1280x960 physical pixels on this machine, about 13 ms).
