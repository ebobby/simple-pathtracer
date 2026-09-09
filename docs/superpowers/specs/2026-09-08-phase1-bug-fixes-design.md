# Phase 1: Bug fixes

First of three phases (bugs, raw speed, image quality). Scope is limited to
correctness problems found in the CPU and GPU renderers.

## Fixes

1. **CPU max depth is honored.** `radiance` recurses only while the current
   depth is below the caller's `max_depth`. Russian roulette still starts
   after depth 5. `max_depth` counts surface interactions: with `max_depth = 1`
   a camera ray returns only the emission of the first surface it hits.

2. **GPU renderer adds no fake light.** The 5% per-bounce ambient term and the
   sky gradient on miss are removed. A miss contributes black, matching the CPU.
   Closed rooms get darker and noisier until phase 3 adds light sampling.

3. **GPU offline pass size.** `SAMPLES_PER_PASS` drops from 500 to 16 so no
   single dispatch runs for seconds (GPU watchdog risk).

4. **Disc texture coordinates.** Both backends build an orthonormal basis
   (Duff et al. 2017) from the disc normal and project the hit offset onto its
   tangent and bitangent: `u = 0.5 + dot(d, t) / (2r)`, `v = 0.5 + dot(d, b) / (2r)`.
   The centre maps to (0.5, 0.5) and the rim to the unit circle inscribed in
   the unit square, for any disc orientation.

5. **GPU sphere precision.** Sphere intersection uses the cancellation-free
   quadratic from Ray Tracing Gems ch. 7 and re-projects the hit point onto
   the sphere surface before computing the normal. Wall spheres of radius 5000
   stay; real planes are a later feature.

## Tests

* `radiance` with a mirror facing a light: black at `max_depth = 1`, the
  light's colour at `max_depth = 2`.
* Disc UV: centre is (0.5, 0.5); rim points lie on the inscribed circle for a
  tilted disc.
* CPU/GPU parity: a small open scene rendered on both backends; the mean
  absolute per-channel difference must be small. Skipped when no GPU adapter
  is available. This requires `render` and `render_gpu` to expose variants that
  return linear pixel buffers instead of writing PNG files.

## Out of scope

Planes, BVH rework, rayon, realtime loop cleanup (phase 2). Light sampling and
denoising (phase 3).
