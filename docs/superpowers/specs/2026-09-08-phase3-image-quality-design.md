# Phase 3: Image quality

Third phase. Goal: far less noise per sample by sampling lights directly,
without introducing bias. Both backends implement the same estimator so the
parity test keeps holding.

## Estimator: next event estimation with multiple importance sampling

At every diffuse (Lambertian) vertex the integrator does two things:

1. **Light sample.** Pick one light uniformly (probability `1/N`), sample a
   direction towards it, trace a shadow ray. If the nearest hit is that light,
   add `throughput * albedo/π * cosθ * Le * w / pdf_light`.
2. **BSDF sample.** Continue the path with a cosine-weighted direction as
   before. If the next hit is a light, its emission is added with weight
   `w` computed from the BSDF pdf and the pdf that light sampling would have
   assigned to the same direction.

`w` is the power heuristic, `a² / (a² + b²)`, in solid-angle measure on both
sides. Camera rays and rays leaving specular vertices add emission at full
weight, because no light sample was taken there.

**Specular materials** (metal, including fuzzy metal, and dielectric) take no
light sample and are treated as delta distributions. Fuzzy-metal reflections of
lights therefore stay as noisy as before but remain unbiased. This is the
standard simplification and keeps the material code small.

## Light sampling

* **Disc**: uniform point on the area. Solid-angle pdf `d² / (A |cosθ_l|)`.
* **Sphere, shading point outside**: uniform direction in the cone subtended
  by the sphere, pdf `1 / (2π (1 - cosθ_max))`.
* **Sphere, shading point inside**: uniform direction over the full sphere,
  pdf `1 / 4π`. Needed for emissive domes.

Lights emit from both sides, as they do today.

## Plumbing

* `Intersection` gains `shape_id`, set by the BVH from its leaf index. The
  BVH collects a light list (shape id plus geometry) when built, and a
  shape-to-light lookup so a BSDF-sampled hit on a light can find its pdf.
* `Scattered` gains `pdf: Option<f64>`: `Some(cosθ/π)` for Lambertian, `None`
  for specular.
* An `Integrator` enum selects `BsdfOnly` (the old estimator, kept as the
  reference implementation) or `NextEventEstimation` (default).
* GPU: lights are a storage buffer of shape indices, `num_lights` replaces the
  unused padding in `RenderParams`, `HitRecord` carries `shape_idx`.

## Depth semantics

`max_depth` still counts surface interactions. At the last allowed
interaction a diffuse vertex still takes its light sample, so `max_depth = 1`
gives direct lighting.

## Tests

* **Analytic**: a Lambertian plane under a disc light of radius `R` at height
  `h` and radiance `L` reflects `albedo * L * R² / (R² + h²)` at the point
  below the centre. Estimate with NEE at `max_depth = 1` and compare.
* **Unbiasedness**: NEE and BSDF-only renders of the same scene must agree in
  mean brightness within noise.
* **Parity**: CPU and GPU NEE renders agree (existing test).

## Measurement

`benches/noise.rs` renders a Cornell-style scene at equal sample counts with
both integrators and reports RMSE against a high-sample reference.

## Out of scope

Low-discrepancy sampling and denoising are candidates for a follow-up once
this lands and is measured.

## Results

`cargo bench --bench noise` on the Cornell box at 80x60, 8 spp x4, depth 20,
RMSE against a 4000 spp x4 reference:

| Integrator | RMSE   |
|------------|--------|
| BSDF only  | 0.198  |
| NEE + MIS  | 0.079  |

Variance falls as 1/N, so NEE needs about 6.3x fewer samples for equal noise
on this scene. Wall-clock per sample was unchanged on the CPU in this run.

Two details found by the tests:

* At the last allowed interaction the light sample takes full weight; with
  the MIS weight applied there the analytic test came out 4% low, because
  the BSDF-sampled share of the light was never traced.
* The parity tolerance was tightened from 3% to 2% mean and 0.05 to 0.03
  mean absolute difference once both backends used NEE.

GPU wall-clock at the examples' unchanged sample counts (Apple M4 Pro):

| Example          | Phase 2 | Phase 3 (NEE) |
|------------------|---------|---------------|
| `cornell --gpu`  | 27.5 s  | 54.4 s        |
| `disco --gpu`    | n/a     | 112.5 s       |

Each diffuse bounce now traces a shadow ray as well, so time per sample
roughly doubles; at equal noise the net gain is still about 3x on the Cornell
box, and the examples' sample counts can be cut accordingly.
