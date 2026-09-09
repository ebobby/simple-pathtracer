# Phase 5: Principled material, environment and sun lighting

## Part A: Principled material

A new `Material::Principled` variant alongside the existing four, so current
scenes keep their look. Parameters: `base_color` (texture), `metallic`,
`roughness`, `transmission`, `ior`, `emission`. Built with
`Principled::new(base_color)` plus builder setters; defaults are a matte
dielectric (metallic 0, roughness 0.5, transmission 0, ior 1.5, no emission).

### Model

GGX with α = roughness², clamped to α ≥ 0.001. Fresnel is Schlick with
`F0 = mix(((ior-1)/(ior+1))², base_color, metallic)`.

Reflection side, a mixture of two lobes:

* specular: `F(wo·h) D(h) G2 / (4 cos_o cos_i)`
* diffuse: `(1 - metallic)(1 - transmission)(1 - F_avg(cos_o)) base_color / π`

Lobe selection uses `p_spec = metallic + (1 - metallic) F_avg(cos_o)`, which
depends on `wo` only, so `eval(wo, wi)` can return the exact mixture pdf
`p_spec pdf_ggx(wi) + (1 - p_spec) cos_i / π`. BSDF sampling weights by
`f cos / pdf`; light sampling uses `eval`. This keeps MIS consistent.

Transmission, chosen with probability `(1 - metallic) transmission`: sample a
visible normal, reflect with probability `F_avg(wo·h)` else refract (total
internal reflection reflects). Both are treated as delta bounces for MIS (no
light sample, next emission hit at full weight) with the visible-normal
weight `G2 / G1` times the per-channel Fresnel ratio. Beer-Lambert absorption
uses `base_color` as transmittance per unit distance inside the object.
Radiance scaling by η² is omitted, as for the existing dielectric.

`D` is computed as `α² / (π (α² cos²θ_h + sin²θ_h)²)` with `sin²θ_h` from
`|h × n|²`, which stays accurate near the normal for small α.

### Facing normals

Lambertian, metal and the principled reflection lobes shade with the normal
flipped to face the incoming ray. Transmission uses the unflipped normal to
tell entering from exiting. The GPU does the same.

### Tests

Sampled pdf equals `eval` pdf; mixture pdf integrates to one over the sphere;
white transmissive sphere in a white furnace returns about one; white rough
diffuse sphere in a furnace stays at or below one; a diffuse disc lit from
its back matches the analytic disc irradiance; NEE versus BSDF-only agreement
with a principled sphere in the scene; CPU/GPU parity with a principled
sphere.

## Part B: Environment and sun

`Scene` gains an `Environment` with an optional sky and an optional sun, and
owns the light list (moved out of the BVH), because infinite lights are not
shapes. `Scene::new(camera, world)` and `with_environment(env)` replace the
struct literal.

* Sky: `Constant(Color)` or `Image` (equirectangular Radiance HDR loaded
  with the `image` crate). Miss rays return the sky radiance. Image skies
  are importance sampled with a marginal/conditional CDF over luminance
  weighted by sin θ; constant skies use uniform sphere directions.
* Sun: direction, radiance and angular radius (default 0.265°). Sampled
  uniformly in its cone; a BSDF-sampled miss inside the cone sees the sun's
  radiance, MIS-weighted like any light.
* Selection power for infinite lights: average radiance times the projected
  area of the scene's bounding sphere.

GPU: light entries use sentinel shape indices for sky and sun; the sky image
and its CDFs are storage buffers; `RenderParams` carries the sun and
constant-sky parameters. `render_gpu` and `render_realtime` keep their
signatures with no environment; `_with_environment` variants take one.

### Tests

Lambertian sphere under a constant sky of radiance 1 reflects exactly its
albedo; image-sky sample pdf equals pdf evaluation and integrates to one;
sun over a diffuse plane matches `albedo L 2π(1 - cos θ_max) / π`; CPU/GPU
parity with sky and sun.

## Out of scope

GPU textures (base color on the GPU stays a flat color), location-aware
light selection, one-sided disc emission.

## Results

Both parts landed with CPU/GPU parity on a scene containing principled
metal and glass spheres, a constant sky and a low sun.

* `examples/outdoor.rs`: 800x600, 200 spp x4, depth 30. GPU 1.0 s, CPU 7.0 s
  on an Apple M4 Pro.
* Noise bench unchanged for existing scenes (Cornell 0.050, uneven lights
  0.102).
* The device limit of eight storage buffers per stage was hit; the sky
  image's marginal and conditional CDFs share one buffer.

Two things worth knowing when using the new pieces:

* Sun brightness is a radiance over a small cone, so the number must be
  large: irradiance is `L * 2π(1 - cos θ)`. For a 0.1 rad cone, `L ≈ 360`
  gives about six times the irradiance of a sky of radiance 0.6.
* Light passing through glass onto other surfaces (caustics) can only be
  found by chance, since light sampling cannot see through the glass. With a
  small bright sun those paths appear as sparse bright dots. A wider cone
  reduces this; a firefly clamp would remove it at the cost of bias and is
  not implemented.
