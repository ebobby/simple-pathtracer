# Phase 4: Quick wins and noise

## Quick wins

1. **Example sample counts** cut to roughly a fifth (a quarter for the
   mirror-heavy scenes, where light sampling helps less) to match the noise
   level the new estimator reaches.
2. **Viewer renders at half resolution while the camera moves.** The
   accumulation buffer keeps its full-resolution size; while moving, the
   tracer runs on a half-size grid and the blit shader upscales with nearest
   sampling. The first still frame restarts accumulation at full resolution.
   Samples per frame adapt separately for the moving and still states.

## Low-discrepancy sampling

Padded 2D Sobol with hash-based Owen scrambling (Burley 2020), the scheme
Cycles uses. Every random decision in a path is a 2D "pair" drawn from the
2D Sobol sequence, scrambled and index-shuffled with a seed hashed from
`(pixel, pair slot)`. This needs no direction-number tables beyond the second
Sobol dimension and has no dimension limit.

Pair slots per path: slot 0 is the pixel jitter; bounce `b` uses slots
`1 + 3b` (BSDF direction), `2 + 3b` (light sample) and `3 + 3b` (x: light
selection or metal fuzz radius or Fresnel; y: Russian roulette). Each material
uses only its own dimensions so slots stay aligned whatever the material.

Metal fuzz stops using a rejection loop and draws a point in the unit ball
from three uniforms directly.

The CPU `radiance_with` and the GPU `trace_path` take a sampler
`(pixel_seed, sample_index)`. The GPU gets `sample_offset` in `RenderParams`
so sample indices continue across passes and frames.

Tests: the 2D sequence is a (0,2)-sequence, so the first 2^k points put
exactly one point in every elementary cell; the shuffled and scrambled
sequence must keep that property. Existing estimator tests keep passing.

## Light selection by power

Lights are picked in proportion to emitted power (mean emission luminance
times area) instead of uniformly. The selection pdf replaces `1/N` in both
the light-sample and BSDF-hit weights. The GPU light buffer becomes
`{shape_idx, select_pdf, cdf}`.

Measured with a second noise-bench scene that has lights of very different
powers.

## Glossy metal with light sampling

Metal fuzz becomes GGX roughness (α = fuzz): visible-normal sampling for the
BSDF direction, Smith height-correlated masking-shadowing, no Fresnel (the
albedo colour plays that role, as before). Fuzz below 0.001 stays a perfect
mirror with no light sample. Rough metal now takes light samples and its
emission hits are MIS-weighted like Lambertian.

Tests: sampled direction pdf equals `pdf()` evaluation; the pdf integrates to
one; the NEE versus BSDF-only agreement test already contains a rough metal
sphere.

## Out of scope

Denoising: Intel Open Image Denoise is not installed on this machine.

## Results

`cargo bench --bench noise`, RMSE at 8 spp x4 against a 4000 spp x4
reference (CPU, Apple M4 Pro):

| Step                          | Cornell box | Uneven lights |
|-------------------------------|-------------|---------------|
| Phase 3 (NEE, white noise)    | 0.0744      | 0.1722        |
| + Sobol sampler               | 0.0601      | 0.1026        |
| + power-weighted selection    | 0.0601      | 0.1018        |
| + GGX metal with light samples| 0.0549      | 0.1018        |

Equal-noise sample ratio versus phase 3: 1.8x on the Cornell box (sampler
plus the rough green sphere now taking light samples) and 2.9x on the
uneven-lights scene from the sampler.

GPU `cornell --gpu` at the cut sample count (500 spp x4): 12.3 s, against
54.4 s at 2500 spp x4 in phase 3.

### Sampler cost

The first sampler version halved CPU throughput (1.91 M to 0.98 M paths/s on
`cargo bench`) and raised GPU cost per sample by 1.6x. Three changes brought
it back to 1.74 M paths/s and 24.5 ms per sample-frame on the GPU Cornell box
(phase 3: 21.8 ms):

* The second Sobol dimension is computed with the Sierpinski butterfly
  (five masked shift-xor steps on the bit-reversed index) instead of a loop
  over the index bits. A test checks it against the direction-number
  recurrence.
* Seeds: deriving the shuffle and scramble seeds by xoring constants into
  one hash was tried and rejected. It raised the uneven-lights RMSE from
  0.102 to 0.124 (the bench is deterministic, so this is not run-to-run
  noise). The three seeds stay independently hashed.
* The light-sample and scalar slots are generated lazily, so specular
  bounces and the first five bounces of any path skip them. Power-weighted selection made no
measurable difference on the uneven-lights scene at depth 20, where indirect
light dominates the residual noise; it is kept because it is cheap, exact,
and covered by tests.

Sampler implementation note: the Laine-Karras permutation only works as an
Owen scramble if every `x ^= x * c` step is a bijection, which needs even
`c`. The stratification test caught an odd constant immediately.

GGX note: the visible-normal pdf integrates to one over the whole sphere,
not over the upper hemisphere. Directions below the horizon are sampled and
discarded; at α = 0.5 they carry about 20% of the mass for a view 37° off
the normal. The pdf test integrates over the full sphere for that reason.
