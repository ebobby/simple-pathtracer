// Path tracer compute shader
// Implements iterative path tracing with BVH traversal

// ============================================================================
// Data Structures
// ============================================================================

struct RenderParams {
    width: u32,
    height: u32,
    samples: u32,
    max_depth: u32,
    frame_seed: u32,
    num_spheres: u32,
    num_discs: u32,
    num_lights: u32,
    sample_offset: u32, // index of this dispatch's first sample per pixel
    sky_type: u32,      // 0 none, 1 constant colour, 2 equirectangular image
    env_width: u32,
    env_height: u32,
    sky_color: vec4<f32>,
    sun_direction: vec4<f32>, // xyz towards the sun, w = cos of angular radius
    sun_radiance: vec4<f32>,  // xyz radiance, w = 1 when present
}

const LIGHT_SKY: u32 = 0xFFFFFFFEu;
const LIGHT_SUN: u32 = 0xFFFFFFFFu;

struct Camera {
    origin: vec4<f32>,
    corner: vec4<f32>,
    horizontal: vec4<f32>,
    vertical: vec4<f32>,
}

struct BVHNode {
    aabb_min: vec4<f32>,
    aabb_max: vec4<f32>,
    left_idx: u32,
    right_idx: u32,
    shape_idx: u32,
    is_leaf: u32,
}

struct Sphere {
    center_radius: vec4<f32>, // xyz = centre, w = radius
    material_idx: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

struct Disc {
    center: vec4<f32>,
    normal: vec4<f32>,
    radius: f32,
    material_idx: u32,
    _pad0: u32,
    _pad1: u32,
}

// Material types
const MATERIAL_LAMBERTIAN: u32 = 0u;
const MATERIAL_METAL: u32 = 1u;
const MATERIAL_DIELECTRIC: u32 = 2u;
const MATERIAL_DIFFUSE_LIGHT: u32 = 3u;

const MATERIAL_PRINCIPLED: u32 = 4u;

struct Material {
    color: vec4<f32>,
    emission: vec4<f32>,
    material_type: u32,
    fuzz: f32, // GGX roughness for metal (as alpha) and principled (as roughness)
    ior: f32,
    metallic: f32,
    transmission: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
}

struct HitRecord {
    p: vec3<f32>,
    t: f32,
    normal: vec3<f32>,
    material_idx: u32,
    shape_idx: u32,
    u: f32,
    v: f32,
    valid: bool,
}

struct ScatterResult {
    ray: Ray,
    attenuation: vec3<f32>,
    pdf: f32,        // solid-angle pdf for non-delta scattering, 0 for specular
    non_delta: bool, // true when the material can take light samples
    valid: bool,
}

struct Light {
    shape_idx: u32,
    select_pdf: f32,
    cdf: f32,
    _pad: u32,
}

struct LightSample {
    direction: vec3<f32>, // unit direction from the shading point
    point: vec3<f32>,     // a point on the light along direction
    pdf: f32,             // solid-angle pdf
    valid: bool,
}

const PI: f32 = 3.14159265359;

// ============================================================================
// Bindings
// ============================================================================

@group(0) @binding(0) var<uniform> params: RenderParams;
@group(0) @binding(1) var<uniform> camera: Camera;
@group(0) @binding(2) var<storage, read> bvh_nodes: array<BVHNode>;
@group(0) @binding(3) var<storage, read> spheres: array<Sphere>;
@group(0) @binding(4) var<storage, read> discs: array<Disc>;
@group(0) @binding(5) var<storage, read> materials: array<Material>;
@group(0) @binding(6) var<storage, read_write> output: array<vec4<f32>>;
@group(0) @binding(7) var<storage, read> lights: array<Light>;
@group(0) @binding(8) var<storage, read> env_pixels: array<vec4<f32>>; // rgb + (u,v) pdf
// Marginal CDF (env_height entries) followed by the conditional CDF rows
@group(0) @binding(9) var<storage, read> env_cdf: array<f32>;

// ============================================================================
// Sampling: padded 2D Sobol with hash-based Owen scrambling
// (mirrors src/sampler.rs)
// ============================================================================

fn hash(x_in: u32) -> u32 {
    var x = x_in;
    x = x ^ (x >> 16u);
    x = x * 0x7feb352du;
    x = x ^ (x >> 15u);
    x = x * 0x846ca68bu;
    x = x ^ (x >> 16u);
    return x;
}

fn hash_combine(seed: u32, value: u32) -> u32 {
    return hash(seed ^ (value * 0x9e3779b9u));
}

// First dimension: bit-reversed index. Second dimension: Pascal-matrix
// (mod 2) product, computed as the Sierpinski butterfly of the reversed index.
fn sobol_2d(index: u32) -> vec2<u32> {
    let x = reverseBits(index);
    var y = x;
    y = y ^ ((y & 0x55555555u) << 1u);
    y = y ^ ((y & 0x33333333u) << 2u);
    y = y ^ ((y & 0x0f0f0f0fu) << 4u);
    y = y ^ ((y & 0x00ff00ffu) << 8u);
    y = y ^ ((y & 0x0000ffffu) << 16u);
    return vec2<u32>(x, y);
}

// Every step is a bijection whose output bits depend only on input bits at
// or below them (even multipliers), which makes the scramble an Owen scramble.
fn laine_karras_permutation(x_in: u32, seed: u32) -> u32 {
    var x = x_in + seed;
    x = x ^ (x * 0x6c50b47cu);
    x = x ^ (x * 0xb82f1e52u);
    x = x ^ (x * 0xc7afe784u);
    x = x ^ (x * 0x8d22f6e6u);
    return x;
}

fn nested_uniform_scramble(x: u32, seed: u32) -> u32 {
    return reverseBits(laine_karras_permutation(reverseBits(x), seed));
}

var<private> sampler_pixel_seed: u32;
var<private> sampler_index: u32;

// The 2D sample for pair `slot` of the current path, both in [0, 1).
fn sample_2d(slot: u32) -> vec2<f32> {
    // Independent seeds for the shuffle and the two scrambles (see sampler.rs)
    let seed = hash_combine(sampler_pixel_seed, slot);
    let index = nested_uniform_scramble(sampler_index, hash_combine(seed, 0u));
    let p = sobol_2d(index);
    let x = nested_uniform_scramble(p.x, hash_combine(seed, 1u));
    let y = nested_uniform_scramble(p.y, hash_combine(seed, 2u));
    return vec2<f32>(f32(x >> 8u), f32(y >> 8u)) / 16777216.0;
}

const SLOT_PIXEL: u32 = 0u;

// First slot of a bounce: BSDF direction, light sample, then
// (light selection / lobe choice / Fresnel, Russian roulette),
// and a secondary BSDF direction.
fn bounce_slot(bounce: u32) -> u32 {
    return 1u + 4u * bounce;
}

// Build orthonormal basis from normal (Duff et al. 2017)
fn build_onb(n: vec3<f32>) -> mat3x3<f32> {
    let sign = select(-1.0, 1.0, n.z >= 0.0);
    let a = -1.0 / (sign + n.z);
    let b = n.x * n.y * a;
    let t = vec3<f32>(1.0 + sign * n.x * n.x * a, sign * b, -sign * n.x);
    let bt = vec3<f32>(b, sign + n.y * n.y * a, -n.y);
    return mat3x3<f32>(t, bt, n);
}

// Cosine-weighted hemisphere sampling from two uniforms
fn random_cosine_direction(normal: vec3<f32>, r1: f32, r2: f32) -> vec3<f32> {
    let phi = 2.0 * PI * r1;
    let sqrt_r2 = sqrt(r2);

    // Local coordinates (z-up hemisphere)
    let x = cos(phi) * sqrt_r2;
    let y = sin(phi) * sqrt_r2;
    let z = sqrt(1.0 - r2);

    // Transform to world space using ONB
    let onb = build_onb(normal);
    return onb * vec3<f32>(x, y, z);
}

// ============================================================================
// Ray Generation
// ============================================================================

fn generate_ray(pixel: vec2<u32>) -> Ray {
    // Stratified jitter within the pixel
    let jitter = sample_2d(SLOT_PIXEL);
    let u = (f32(pixel.x) + jitter.x) / f32(params.width);
    let v = (f32(pixel.y) + jitter.y) / f32(params.height);

    let origin = camera.origin.xyz;
    let direction = camera.corner.xyz + camera.horizontal.xyz * u + camera.vertical.xyz * v - origin;

    return Ray(origin, direction);
}

// ============================================================================
// Intersection Testing
// ============================================================================

// Slab test. Returns the (positive) entry distance, or -1.0 on a miss.
fn intersect_aabb(origin: vec3<f32>, inv_dir: vec3<f32>, aabb_min: vec3<f32>, aabb_max: vec3<f32>, t_max: f32) -> f32 {
    let t0 = (aabb_min - origin) * inv_dir;
    let t1 = (aabb_max - origin) * inv_dir;

    let tmin = min(t0, t1);
    let tmax = max(t0, t1);

    let t_enter = max(max(tmin.x, tmin.y), max(tmin.z, 0.0001));
    let t_exit = min(min(tmax.x, tmax.y), min(tmax.z, t_max));

    return select(-1.0, t_enter, t_enter <= t_exit);
}

fn intersect_sphere(ray: Ray, sphere: Sphere, t_min: f32, t_max: f32) -> HitRecord {
    var hit: HitRecord;
    hit.valid = false;

    let center = sphere.center_radius.xyz;
    let radius = sphere.center_radius.w;

    // Cancellation-free quadratic (Ray Tracing Gems, ch. 7): the discriminant
    // is computed from the perpendicular distance between the ray and the
    // sphere centre instead of dot(oc, oc) - r*r, which loses all precision
    // when |oc| is much larger than r (e.g. radius 5000 wall spheres).
    let oc = center - ray.origin;
    let a = dot(ray.direction, ray.direction);
    let b = dot(oc, ray.direction);
    let perp = oc - (b / a) * ray.direction;
    let discriminant = radius * radius - dot(perp, perp);

    if discriminant < 0.0 {
        return hit;
    }

    let sqrtd = sqrt(a * discriminant);
    let q = select(b - sqrtd, b + sqrtd, b > 0.0);
    if q == 0.0 {
        // Grazing tangent hit through the origin's foot point; treat as a miss.
        return hit;
    }
    let c = dot(oc, oc) - radius * radius;
    let t_near = c / q;
    let t_far = q / a;

    var root = min(t_near, t_far);
    if root < t_min || root > t_max {
        root = max(t_near, t_far);
        if root < t_min || root > t_max {
            return hit;
        }
    }

    hit.t = root;
    hit.normal = normalize(ray.origin + ray.direction * root - center);
    // Re-project onto the sphere surface so the hit point does not drift
    // off the surface for large spheres.
    hit.p = center + hit.normal * radius;
    hit.material_idx = sphere.material_idx;
    hit.valid = true;

    // Compute UV coordinates (spherical mapping)
    let d = hit.normal;
    hit.u = 0.5 + atan2(d.z, d.x) / (2.0 * PI);
    hit.v = 0.5 - asin(d.y) / PI;

    return hit;
}

fn intersect_disc(ray: Ray, disc: Disc, t_min: f32, t_max: f32) -> HitRecord {
    var hit: HitRecord;
    hit.valid = false;

    let center = disc.center.xyz;
    let normal = disc.normal.xyz;
    let radius = disc.radius;

    let denom = dot(normal, ray.direction);
    if abs(denom) < 0.0001 {
        return hit;
    }

    let t = dot(center - ray.origin, normal) / denom;
    if t < t_min || t > t_max {
        return hit;
    }

    let p = ray.origin + ray.direction * t;
    let d = p - center;
    if dot(d, d) > radius * radius {
        return hit;
    }

    hit.t = t;
    hit.p = p;
    hit.normal = normal;
    hit.material_idx = disc.material_idx;
    hit.valid = true;
    let onb = build_onb(normal);
    hit.u = 0.5 + dot(d, onb[0]) / (2.0 * radius);
    hit.v = 0.5 + dot(d, onb[1]) / (2.0 * radius);

    return hit;
}

fn intersect_shape(ray: Ray, shape_idx: u32, t_max: f32) -> HitRecord {
    if shape_idx < params.num_spheres {
        return intersect_sphere(ray, spheres[shape_idx], 0.0001, t_max);
    }
    var miss: HitRecord;
    miss.valid = false;
    let disc_idx = shape_idx - params.num_spheres;
    if disc_idx < params.num_discs {
        return intersect_disc(ray, discs[disc_idx], 0.0001, t_max);
    }
    return miss;
}

fn intersect_bvh(ray: Ray) -> HitRecord {
    var closest: HitRecord;
    closest.valid = false;
    closest.t = 1e30;

    // Inverse direction computed once per ray for every slab test.
    let inv_dir = 1.0 / ray.direction;

    // Explicit stack for iterative traversal. Each node's box is tested when
    // it is popped; testing both children at the parent was measured slower
    // on the GPU because it doubles node loads.
    var stack: array<u32, 32>;
    var stack_ptr: i32 = 0;

    stack[0] = 0u;
    stack_ptr = 1;

    while stack_ptr > 0 {
        stack_ptr = stack_ptr - 1;
        let node = bvh_nodes[stack[stack_ptr]];

        if intersect_aabb(ray.origin, inv_dir, node.aabb_min.xyz, node.aabb_max.xyz, closest.t) < 0.0 {
            continue;
        }

        if node.is_leaf == 1u {
            var hit = intersect_shape(ray, node.shape_idx, closest.t);
            if hit.valid && hit.t < closest.t {
                hit.shape_idx = node.shape_idx;
                closest = hit;
            }
        } else {
            stack[stack_ptr] = node.left_idx;
            stack[stack_ptr + 1] = node.right_idx;
            stack_ptr = stack_ptr + 2;
        }
    }

    return closest;
}

// ============================================================================
// GGX microfacet reflection (mirrors src/material/ggx.rs)
// ============================================================================

const GGX_MIN_ALPHA: f32 = 1e-3;

// D(h) with sin^2 from |h x n|^2, which stays accurate near the normal.
fn ggx_distribution(h: vec3<f32>, normal: vec3<f32>, alpha: f32) -> f32 {
    let cos_h = dot(h, normal);
    if cos_h <= 0.0 {
        return 0.0;
    }
    let a2 = alpha * alpha;
    let c = cross(h, normal);
    let sin2_h = dot(c, c);
    let t = a2 * cos_h * cos_h + sin2_h;
    return a2 / (PI * t * t);
}

fn ggx_lambda(cos_theta: f32, alpha: f32) -> f32 {
    let cos2 = cos_theta * cos_theta;
    let tan2 = max(1.0 - cos2, 0.0) / cos2;
    return (-1.0 + sqrt(1.0 + alpha * alpha * tan2)) * 0.5;
}

fn ggx_g1(cos_theta: f32, alpha: f32) -> f32 {
    return 1.0 / (1.0 + ggx_lambda(cos_theta, alpha));
}

fn ggx_g2(cos_o: f32, cos_i: f32, alpha: f32) -> f32 {
    return 1.0 / (1.0 + ggx_lambda(cos_o, alpha) + ggx_lambda(cos_i, alpha));
}

// Sample a visible microfacet normal in the local frame (z = normal).
fn ggx_sample_visible_normal(wo: vec3<f32>, alpha: f32, u1: f32, u2: f32) -> vec3<f32> {
    let vh = normalize(vec3<f32>(alpha * wo.x, alpha * wo.y, wo.z));
    let len_sq = vh.x * vh.x + vh.y * vh.y;
    var t1 = vec3<f32>(1.0, 0.0, 0.0);
    if len_sq > 0.0 {
        t1 = vec3<f32>(-vh.y, vh.x, 0.0) / sqrt(len_sq);
    }
    let t2 = cross(vh, t1);

    let r = sqrt(u1);
    let phi = 2.0 * PI * u2;
    let p1 = r * cos(phi);
    var p2 = r * sin(phi);
    let s = 0.5 * (1.0 + vh.z);
    p2 = (1.0 - s) * sqrt(max(1.0 - p1 * p1, 0.0)) + s * p2;

    let nh = t1 * p1 + t2 * p2 + vh * sqrt(max(1.0 - p1 * p1 - p2 * p2, 0.0));
    return normalize(vec3<f32>(alpha * nh.x, alpha * nh.y, max(nh.z, 0.0)));
}

// Visible-normal-sampling pdf of wi from wo, over the whole sphere.
fn ggx_pdf(alpha: f32, wo: vec3<f32>, wi: vec3<f32>, normal: vec3<f32>) -> f32 {
    let cos_o = dot(wo, normal);
    if cos_o <= 0.0 {
        return 0.0;
    }
    let h = normalize(wo + wi);
    if dot(wo, h) <= 0.0 {
        return 0.0;
    }
    return ggx_g1(cos_o, alpha) * ggx_distribution(h, normal, alpha) / (4.0 * cos_o);
}

// BRDF value without albedo (x) and pdf (y); both zero when invalid.
fn ggx_eval(alpha: f32, wo: vec3<f32>, wi: vec3<f32>, normal: vec3<f32>) -> vec2<f32> {
    let cos_o = dot(wo, normal);
    let cos_i = dot(wi, normal);
    if cos_o <= 0.0 || cos_i <= 0.0 {
        return vec2<f32>(0.0);
    }
    let h = normalize(wo + wi);
    if dot(wo, h) <= 0.0 {
        return vec2<f32>(0.0);
    }
    let d = ggx_distribution(h, normal, alpha);
    let f = d * ggx_g2(cos_o, cos_i, alpha) / (4.0 * cos_o * cos_i);
    return vec2<f32>(f, ggx_pdf(alpha, wo, wi, normal));
}

// ---- Principled material (mirrors src/material/principled.rs) ----

fn principled_alpha(material: Material) -> f32 {
    return max(material.fuzz * material.fuzz, GGX_MIN_ALPHA);
}

fn principled_f0(material: Material, base: vec3<f32>) -> vec3<f32> {
    let d = (material.ior - 1.0) / (material.ior + 1.0);
    return mix(vec3<f32>(d * d), base, material.metallic);
}

fn fresnel_schlick(f0: vec3<f32>, cos_theta: f32) -> vec3<f32> {
    let w = pow(1.0 - clamp(cos_theta, 0.0, 1.0), 5.0);
    return f0 + (vec3<f32>(1.0) - f0) * w;
}

fn average3(c: vec3<f32>) -> f32 {
    return (c.x + c.y + c.z) / 3.0;
}

fn principled_p_transmit(material: Material) -> f32 {
    return (1.0 - material.metallic) * material.transmission;
}

fn principled_p_specular(material: Material, f0: vec3<f32>, cos_o: f32) -> f32 {
    return material.metallic + (1.0 - material.metallic) * average3(fresnel_schlick(f0, cos_o));
}

// Opaque-branch BRDF (rgb) and mixture pdf (w), scaled by the branch
// probability; zero when either direction is below the facing normal.
fn principled_eval_opaque(material: Material, wo: vec3<f32>, wi: vec3<f32>, n: vec3<f32>) -> vec4<f32> {
    let cos_o = dot(wo, n);
    let cos_i = dot(wi, n);
    if cos_o <= 0.0 || cos_i <= 0.0 {
        return vec4<f32>(0.0);
    }
    let base = material.color.xyz;
    let alpha = principled_alpha(material);
    let f0 = principled_f0(material, base);
    let e = ggx_eval(alpha, wo, wi, n);
    if e.y <= 0.0 {
        return vec4<f32>(0.0);
    }
    let h = normalize(wo + wi);
    let fresnel_h = fresnel_schlick(f0, dot(wo, h));
    let fresnel_avg = average3(fresnel_schlick(f0, cos_o));
    let diffuse_weight = (1.0 - material.metallic) * (1.0 - fresnel_avg);

    let f = fresnel_h * e.x + base * (diffuse_weight / PI);
    let p_spec = principled_p_specular(material, f0, cos_o);
    let pdf = p_spec * e.y + (1.0 - p_spec) * cos_i / PI;
    let p_opaque = 1.0 - principled_p_transmit(material);
    return vec4<f32>(f * p_opaque, pdf * p_opaque);
}

// BRDF value (rgb) and pdf (w) for reflecting wo into wi at hit; pdf is zero
// for delta materials and for directions below the surface. `normal` must
// already face wo.
fn bsdf_eval(material: Material, wo: vec3<f32>, wi: vec3<f32>, normal: vec3<f32>) -> vec4<f32> {
    switch material.material_type {
        case MATERIAL_LAMBERTIAN: {
            let cos_i = dot(wi, normal);
            if cos_i <= 0.0 || dot(wo, normal) <= 0.0 {
                return vec4<f32>(0.0);
            }
            return vec4<f32>(material.color.xyz / PI, cos_i / PI);
        }
        case MATERIAL_METAL: {
            if material.fuzz < GGX_MIN_ALPHA {
                return vec4<f32>(0.0);
            }
            let e = ggx_eval(material.fuzz, wo, wi, normal);
            return vec4<f32>(material.color.xyz * e.x, e.y);
        }
        case MATERIAL_PRINCIPLED: {
            return principled_eval_opaque(material, wo, wi, normal);
        }
        default: {
            return vec4<f32>(0.0);
        }
    }
}

// ============================================================================
// Material Scattering
// ============================================================================

fn reflect(v: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    return v - 2.0 * dot(v, n) * n;
}

fn refract(uv: vec3<f32>, n: vec3<f32>, etai_over_etat: f32) -> vec3<f32> {
    let cos_theta = min(dot(-uv, n), 1.0);
    let r_out_perp = etai_over_etat * (uv + cos_theta * n);
    let r_out_parallel = -sqrt(abs(1.0 - dot(r_out_perp, r_out_perp))) * n;
    return r_out_perp + r_out_parallel;
}

fn schlick(cosine: f32, ref_idx: f32) -> f32 {
    var r0 = (1.0 - ref_idx) / (1.0 + ref_idx);
    r0 = r0 * r0;
    return r0 + (1.0 - r0) * pow(1.0 - cosine, 5.0);
}

// u: two uniforms for the direction and one for a scalar decision
// u: two uniforms for a microfacet/primary direction and one scalar choice;
// u2: two uniforms for a secondary direction (principled diffuse lobe).
fn scatter(ray: Ray, hit: HitRecord, material: Material, u: vec3<f32>, u2: vec2<f32>) -> ScatterResult {
    var result: ScatterResult;
    result.valid = false;
    result.non_delta = false;
    result.pdf = 0.0;

    let wo = -normalize(ray.direction);
    // Reflection lobes shade with the normal facing the viewer
    let entering = dot(wo, hit.normal) >= 0.0;
    let n = select(-hit.normal, hit.normal, entering);

    // Offset origin along the facing normal to prevent self-intersection
    let offset_origin = hit.p + n * 0.001;

    switch material.material_type {
        case MATERIAL_LAMBERTIAN: {
            // Cosine-weighted diffuse scattering (matches CPU implementation)
            let direction = random_cosine_direction(n, u.x, u.y);
            result.ray = Ray(offset_origin, direction);
            result.attenuation = material.color.xyz;
            result.pdf = max(dot(direction, n), 0.0) / PI;
            result.non_delta = true;
            result.valid = true;
        }
        case MATERIAL_METAL: {
            // fuzz is the GGX roughness; below GGX_MIN_ALPHA it is a mirror
            if material.fuzz < GGX_MIN_ALPHA {
                let reflected = reflect(-wo, n);
                if dot(reflected, n) > 0.0 {
                    result.ray = Ray(offset_origin, reflected);
                    result.attenuation = material.color.xyz;
                    result.valid = true;
                }
            } else {
                let cos_o = dot(wo, n);
                if cos_o > 0.0 {
                    let onb = build_onb(n);
                    let wo_local = vec3<f32>(dot(wo, onb[0]), dot(wo, onb[1]), cos_o);
                    let h = onb * ggx_sample_visible_normal(wo_local, material.fuzz, u.x, u.y);
                    let wi = h * (2.0 * dot(wo, h)) - wo;
                    let cos_i = dot(wi, n);
                    let e = ggx_eval(material.fuzz, wo, wi, n);
                    if cos_i > 0.0 && e.y > 0.0 {
                        result.ray = Ray(offset_origin, wi);
                        // f * cos / pdf, which for visible-normal sampling is G2 / G1
                        result.attenuation = material.color.xyz * (e.x * cos_i / e.y);
                        result.pdf = e.y;
                        result.non_delta = true;
                        result.valid = true;
                    }
                }
            }
        }
        case MATERIAL_DIELECTRIC: {
            // Glass with refraction
            result.attenuation = material.color.xyz;
            let unit_direction = normalize(ray.direction);
            let d = dot(unit_direction, hit.normal);

            var outward_normal: vec3<f32>;
            var ni_over_nt: f32;
            var cosine: f32;

            if d > 0.0 {
                // Ray exiting glass (hitting from inside)
                outward_normal = -hit.normal;
                ni_over_nt = material.ior;
                cosine = material.ior * d; // d is already a cosine (unit direction)
            } else {
                // Ray entering glass (hitting from outside)
                outward_normal = hit.normal;
                ni_over_nt = 1.0 / material.ior;
                cosine = -d;
            }

            let cos_theta = min(dot(-unit_direction, outward_normal), 1.0);
            let sin_theta = sqrt(1.0 - cos_theta * cos_theta);
            let cannot_refract = ni_over_nt * sin_theta > 1.0;

            var direction: vec3<f32>;
            if cannot_refract || schlick(cosine, material.ior) > u.z {
                // Reflection - offset away from surface (along outward_normal)
                direction = reflect(unit_direction, outward_normal);
                result.ray = Ray(hit.p + outward_normal * 0.001, direction);
            } else {
                // Refraction - offset into the material we're entering (opposite of outward_normal)
                direction = refract(unit_direction, outward_normal, ni_over_nt);
                result.ray = Ray(hit.p - outward_normal * 0.001, direction);
            }
            result.valid = true;
        }
        case MATERIAL_DIFFUSE_LIGHT: {
            // Lights don't scatter
            result.valid = false;
        }
        case MATERIAL_PRINCIPLED: {
            let cos_o = dot(wo, n);
            if cos_o <= 0.0 {
                return result;
            }
            let base = material.color.xyz;
            let alpha = principled_alpha(material);
            let f0 = principled_f0(material, base);
            let onb = build_onb(n);
            let wo_local = vec3<f32>(dot(wo, onb[0]), dot(wo, onb[1]), cos_o);

            let p_transmit = principled_p_transmit(material);
            var choice = u.z;

            if choice < p_transmit {
                // Glass branch: Fresnel reflection or refraction about a
                // sampled microfacet normal, treated as delta for MIS.
                choice = choice / p_transmit;
                let h = onb * ggx_sample_visible_normal(wo_local, alpha, u.x, u.y);
                let cos_oh = dot(wo, h);
                let fresnel = fresnel_schlick(f0, cos_oh);
                let fresnel_avg = average3(fresnel);
                let eta = select(material.ior, 1.0 / material.ior, entering);
                let sin2_t = eta * eta * (1.0 - cos_oh * cos_oh);
                let tir = sin2_t >= 1.0;

                if tir || choice < fresnel_avg {
                    let wi = h * (2.0 * cos_oh) - wo;
                    let cos_i = dot(wi, n);
                    if cos_i <= 0.0 {
                        return result;
                    }
                    let weight = ggx_g2(cos_o, cos_i, alpha) / ggx_g1(cos_o, alpha);
                    var tint = fresnel / fresnel_avg;
                    if tir {
                        tint = vec3<f32>(1.0);
                    }
                    result.ray = Ray(offset_origin, wi);
                    result.attenuation = tint * weight;
                    result.valid = true;
                    return result;
                }

                let cos_t = sqrt(1.0 - sin2_t);
                let wi = -wo * eta + h * (eta * cos_oh - cos_t);
                let cos_i = -dot(wi, n);
                if cos_i <= 0.0 {
                    return result;
                }
                let weight = ggx_g2(cos_o, cos_i, alpha) / ggx_g1(cos_o, alpha);
                let tint = (vec3<f32>(1.0) - fresnel) / (1.0 - fresnel_avg);
                result.ray = Ray(hit.p - n * 0.001, wi);
                result.attenuation = tint * weight;
                result.valid = true;
                return result;
            }

            // Opaque branch: specular or diffuse lobe, weighted by f cos / pdf
            choice = (choice - p_transmit) / (1.0 - p_transmit);
            let p_spec = principled_p_specular(material, f0, cos_o);
            var wi: vec3<f32>;
            if choice < p_spec {
                let h = onb * ggx_sample_visible_normal(wo_local, alpha, u.x, u.y);
                wi = h * (2.0 * dot(wo, h)) - wo;
            } else {
                wi = random_cosine_direction(n, u2.x, u2.y);
            }
            let cos_i = dot(wi, n);
            if cos_i <= 0.0 {
                return result;
            }
            let e = principled_eval_opaque(material, wo, wi, n);
            if e.w <= 0.0 {
                return result;
            }
            result.ray = Ray(offset_origin, wi);
            result.attenuation = e.xyz * (cos_i / e.w);
            result.pdf = e.w;
            result.non_delta = true;
            result.valid = true;
        }
        default: {
            result.valid = false;
        }
    }

    return result;
}

// ============================================================================
// Environment (mirrors src/environment.rs)
// ============================================================================

fn env_texel(u: f32, v: f32) -> u32 {
    let x = min(u32(u * f32(params.env_width)), params.env_width - 1u);
    let y = min(u32(v * f32(params.env_height)), params.env_height - 1u);
    return y * params.env_width + x;
}

fn env_direction_to_uv(d: vec3<f32>) -> vec2<f32> {
    let theta = acos(clamp(d.y, -1.0, 1.0));
    var phi = atan2(d.z, d.x);
    if phi < 0.0 {
        phi = phi + 2.0 * PI;
    }
    return vec2<f32>(phi / (2.0 * PI), theta / PI);
}

fn env_uv_to_direction(u: f32, v: f32) -> vec3<f32> {
    let theta = PI * v;
    let phi = 2.0 * PI * u;
    let sin_theta = sin(theta);
    return vec3<f32>(sin_theta * cos(phi), cos(theta), sin_theta * sin(phi));
}

fn sky_radiance(direction: vec3<f32>) -> vec3<f32> {
    if params.sky_type == 1u {
        return params.sky_color.xyz;
    }
    if params.sky_type == 2u {
        let uv = env_direction_to_uv(direction);
        return env_pixels[env_texel(uv.x, uv.y)].xyz;
    }
    return vec3<f32>(0.0);
}

fn sky_pdf(direction: vec3<f32>) -> f32 {
    if params.sky_type == 1u {
        return 1.0 / (4.0 * PI);
    }
    if params.sky_type == 2u {
        let uv = env_direction_to_uv(direction);
        let sin_theta = sin(PI * uv.y);
        if sin_theta <= 0.0 {
            return 0.0;
        }
        return env_pixels[env_texel(uv.x, uv.y)].w / (2.0 * PI * PI * sin_theta);
    }
    return 0.0;
}

// Importance-sampled sky direction (xyz) and its pdf (w).
fn sky_sample(u1: f32, u2: f32) -> vec4<f32> {
    if params.sky_type == 1u {
        let z = 1.0 - 2.0 * u1;
        let r = sqrt(max(1.0 - z * z, 0.0));
        let phi = 2.0 * PI * u2;
        return vec4<f32>(r * cos(phi), r * sin(phi), z, 1.0 / (4.0 * PI));
    }

    // Row by the marginal CDF
    var lo = 0u;
    var hi = params.env_height - 1u;
    while lo < hi {
        let mid = (lo + hi) / 2u;
        if env_cdf[mid] <= u1 {
            lo = mid + 1u;
        } else {
            hi = mid;
        }
    }
    let y = lo;
    var row_start = 0.0;
    if y > 0u {
        row_start = env_cdf[y - 1u];
    }
    let row_span = env_cdf[y] - row_start;
    var v_in_row = 0.5;
    if row_span > 0.0 {
        v_in_row = clamp((u1 - row_start) / row_span, 0.0, 0.999999);
    }

    // Column by the row's conditional CDF
    let row_base = y * params.env_width;
    let cond_base = params.env_height + row_base;
    lo = 0u;
    hi = params.env_width - 1u;
    while lo < hi {
        let mid = (lo + hi) / 2u;
        if env_cdf[cond_base + mid] <= u2 {
            lo = mid + 1u;
        } else {
            hi = mid;
        }
    }
    let x = lo;
    var col_start = 0.0;
    if x > 0u {
        col_start = env_cdf[cond_base + x - 1u];
    }
    let col_span = env_cdf[cond_base + x] - col_start;
    var u_in_col = 0.5;
    if col_span > 0.0 {
        u_in_col = clamp((u2 - col_start) / col_span, 0.0, 0.999999);
    }

    let u = (f32(x) + u_in_col) / f32(params.env_width);
    let v = (f32(y) + v_in_row) / f32(params.env_height);
    let direction = env_uv_to_direction(u, v);
    let pdf = env_pixels[row_base + x].w / (2.0 * PI * PI * sin(PI * v));
    return vec4<f32>(direction, pdf);
}

fn sun_pdf() -> f32 {
    return 1.0 / (2.0 * PI * (1.0 - params.sun_direction.w));
}

fn sun_contains(direction: vec3<f32>) -> bool {
    return params.sun_radiance.w > 0.0 && dot(direction, params.sun_direction.xyz) >= params.sun_direction.w;
}

fn sun_sample(u1: f32, u2: f32) -> vec3<f32> {
    let cos_max = params.sun_direction.w;
    let cos_theta = 1.0 - u1 * (1.0 - cos_max);
    let sin_theta = sqrt(max(1.0 - cos_theta * cos_theta, 0.0));
    let phi = 2.0 * PI * u2;
    let onb = build_onb(params.sun_direction.xyz);
    return onb * vec3<f32>(sin_theta * cos(phi), sin_theta * sin(phi), cos_theta);
}

// ============================================================================
// Light Sampling (mirrors src/light.rs)
// ============================================================================

fn power_heuristic(pdf_a: f32, pdf_b: f32) -> f32 {
    let a = pdf_a * pdf_a;
    let b = pdf_b * pdf_b;
    if a + b == 0.0 {
        return 0.0;
    }
    return a / (a + b);
}

// Sample a direction from p towards the light with the given shape index.
fn light_sample(shape_idx: u32, p: vec3<f32>, u1: f32, u2: f32) -> LightSample {
    var s: LightSample;
    s.valid = false;

    if shape_idx == LIGHT_SKY {
        let sample = sky_sample(u1, u2);
        s.direction = sample.xyz;
        s.point = p + s.direction;
        s.pdf = sample.w;
        s.valid = sample.w > 0.0;
        return s;
    }
    if shape_idx == LIGHT_SUN {
        s.direction = sun_sample(u1, u2);
        s.point = p + s.direction;
        s.pdf = sun_pdf();
        s.valid = true;
        return s;
    }

    let phi = 2.0 * PI * u2;

    if shape_idx < params.num_spheres {
        let sphere = spheres[shape_idx];
        let center = sphere.center_radius.xyz;
        let radius = sphere.center_radius.w;
        let to_center = center - p;
        let dist_sq = dot(to_center, to_center);

        if dist_sq <= radius * radius {
            // Inside the sphere: every direction hits it.
            let z = 1.0 - 2.0 * u1;
            let r = sqrt(max(1.0 - z * z, 0.0));
            s.direction = vec3<f32>(r * cos(phi), r * sin(phi), z);
            s.point = p + s.direction;
            s.pdf = 1.0 / (4.0 * PI);
            s.valid = true;
            return s;
        }

        // Outside: uniform direction inside the subtended cone.
        let cos_theta_max = sqrt(1.0 - radius * radius / dist_sq);
        let cos_theta = 1.0 - u1 * (1.0 - cos_theta_max);
        let sin_theta = sqrt(max(1.0 - cos_theta * cos_theta, 0.0));
        let axis = to_center / sqrt(dist_sq);
        let onb = build_onb(axis);
        s.direction = onb * vec3<f32>(sin_theta * cos(phi), sin_theta * sin(phi), cos_theta);
        s.point = p + s.direction;
        s.pdf = 1.0 / (2.0 * PI * (1.0 - cos_theta_max));
        s.valid = true;
        return s;
    }

    let disc = discs[shape_idx - params.num_spheres];
    let center = disc.center.xyz;
    let normal = disc.normal.xyz;
    let radius = disc.radius;
    let onb = build_onb(normal);
    let r = radius * sqrt(u1);
    let point = center + onb[0] * (r * cos(phi)) + onb[1] * (r * sin(phi));

    let to_light = point - p;
    let dist_sq = dot(to_light, to_light);
    if dist_sq == 0.0 {
        return s;
    }
    let direction = to_light / sqrt(dist_sq);
    let cos_light = abs(dot(direction, normal));
    if cos_light < 1e-6 {
        return s;
    }
    s.direction = direction;
    s.point = point;
    s.pdf = dist_sq / (PI * radius * radius * cos_light);
    s.valid = true;
    return s;
}

// Solid-angle pdf that light_sample from p would produce the unit direction
// that reaches the light at point.
fn light_pdf(shape_idx: u32, p: vec3<f32>, point: vec3<f32>, direction: vec3<f32>) -> f32 {
    if shape_idx == LIGHT_SKY {
        return sky_pdf(direction);
    }
    if shape_idx == LIGHT_SUN {
        if sun_contains(direction) {
            return sun_pdf();
        }
        return 0.0;
    }
    if shape_idx < params.num_spheres {
        let sphere = spheres[shape_idx];
        let center = sphere.center_radius.xyz;
        let radius = sphere.center_radius.w;
        let to_center = center - p;
        let dist_sq = dot(to_center, to_center);
        if dist_sq <= radius * radius {
            return 1.0 / (4.0 * PI);
        }
        let cos_theta_max = sqrt(1.0 - radius * radius / dist_sq);
        return 1.0 / (2.0 * PI * (1.0 - cos_theta_max));
    }

    let disc = discs[shape_idx - params.num_spheres];
    let normal = disc.normal.xyz;
    let radius = disc.radius;
    let to_point = point - p;
    let cos_light = abs(dot(direction, normal));
    if cos_light < 1e-6 {
        return 0.0;
    }
    return dot(to_point, to_point) / (PI * radius * radius * cos_light);
}

// Selection probability of the light that is shape shape_idx (0 if none).
fn light_select_pdf(shape_idx: u32) -> f32 {
    for (var i = 0u; i < params.num_lights; i = i + 1u) {
        if lights[i].shape_idx == shape_idx {
            return lights[i].select_pdf;
        }
    }
    return 0.0;
}

// Direct lighting at a non-delta vertex seen from wo, from one light chosen
// in proportion to its power, weighted against BSDF sampling unless
// full_weight is set.
fn sample_direct_light(hit: HitRecord, material: Material, wo: vec3<f32>, full_weight: bool, u_light: vec2<f32>, u_select: f32) -> vec3<f32> {
    var light_index = params.num_lights - 1u;
    for (var i = 0u; i < params.num_lights; i = i + 1u) {
        if u_select < lights[i].cdf {
            light_index = i;
            break;
        }
    }
    let light = lights[light_index];
    let light_shape = light.shape_idx;

    let ls = light_sample(light_shape, hit.p, u_light.x, u_light.y);
    if !ls.valid {
        return vec3<f32>(0.0);
    }
    let n = select(-hit.normal, hit.normal, dot(wo, hit.normal) >= 0.0);
    let cos_theta = dot(ls.direction, n);
    let bsdf = bsdf_eval(material, wo, ls.direction, n);
    if bsdf.w <= 0.0 {
        return vec3<f32>(0.0);
    }

    let shadow_ray = Ray(hit.p + n * 0.001, ls.direction);
    let shadow_hit = intersect_bvh(shadow_ray);
    var emitted = vec3<f32>(0.0);
    if light_shape == LIGHT_SKY || light_shape == LIGHT_SUN {
        // Infinite lights: the shadow ray must leave the scene
        if shadow_hit.valid {
            return vec3<f32>(0.0);
        }
        if light_shape == LIGHT_SKY {
            emitted = sky_radiance(ls.direction);
        } else {
            emitted = params.sun_radiance.xyz;
        }
    } else {
        // Shape lights: the nearest hit must be that shape
        if !shadow_hit.valid || shadow_hit.shape_idx != light_shape {
            return vec3<f32>(0.0);
        }
        emitted = materials[shadow_hit.material_idx].emission.xyz;
    }
    let pdf_light = ls.pdf * light.select_pdf;
    var weight = 1.0;
    if !full_weight {
        weight = power_heuristic(pdf_light, bsdf.w);
    }

    return bsdf.xyz * emitted * (cos_theta / pdf_light * weight);
}

// ============================================================================
// Path Tracing
// ============================================================================

fn trace_path(initial_ray: Ray) -> vec3<f32> {
    var ray = initial_ray;
    var color = vec3<f32>(0.0);
    var throughput = vec3<f32>(1.0);

    let use_nee = params.num_lights > 0u;

    // How the current ray was generated: camera and specular bounces add any
    // emission they find at full weight; diffuse bounces carry their pdf so
    // emission can be weighted against light sampling.
    var prev_specular = true;
    var prev_pdf = 0.0;

    for (var depth: u32 = 0u; depth < params.max_depth; depth = depth + 1u) {
        let hit = intersect_bvh(ray);

        if !hit.valid {
            // Left the scene: sky and sun, each MIS-weighted against the
            // chance that light sampling would have picked this direction.
            let direction = normalize(ray.direction);
            if params.sky_type != 0u {
                var weight = 1.0;
                if use_nee && !prev_specular {
                    weight = power_heuristic(prev_pdf, sky_pdf(direction) * light_select_pdf(LIGHT_SKY));
                }
                color = color + throughput * sky_radiance(direction) * weight;
            }
            if sun_contains(direction) {
                var weight = 1.0;
                if use_nee && !prev_specular {
                    weight = power_heuristic(prev_pdf, sun_pdf() * light_select_pdf(LIGHT_SUN));
                }
                color = color + throughput * params.sun_radiance.xyz * weight;
            }
            break;
        }

        let material = materials[hit.material_idx];

        // Beer-Lambert absorption when leaving a transmissive principled object
        if material.material_type == MATERIAL_PRINCIPLED && material.transmission > 0.0
            && dot(ray.direction, hit.normal) > 0.0 {
            throughput = throughput * pow(max(material.color.xyz, vec3<f32>(0.0)), vec3<f32>(hit.t));
        }

        // Add emission
        let emitted = material.emission.xyz;
        if emitted.x > 0.0 || emitted.y > 0.0 || emitted.z > 0.0 {
            var weight = 1.0;
            if use_nee && !prev_specular {
                let direction = normalize(ray.direction);
                let pdf_light = light_pdf(hit.shape_idx, ray.origin, hit.p, direction) * light_select_pdf(hit.shape_idx);
                weight = power_heuristic(prev_pdf, pdf_light);
            }
            color = color + throughput * emitted * weight;
        }
        if material.material_type == MATERIAL_DIFFUSE_LIGHT {
            // Lights don't scatter
            break;
        }

        // Sample slots for this bounce; the scalar slot is generated only
        // when something consumes it.
        let slot = bounce_slot(depth);
        let u_bsdf = sample_2d(slot);
        var u_scalar = vec2<f32>(0.0);
        var have_scalar = false;
        var u_secondary = vec2<f32>(0.0);
        if material.material_type == MATERIAL_DIELECTRIC || material.material_type == MATERIAL_PRINCIPLED {
            u_scalar = sample_2d(slot + 2u);
            have_scalar = true;
        }
        if material.material_type == MATERIAL_PRINCIPLED {
            u_secondary = sample_2d(slot + 3u);
        }

        // Scatter ray
        let scattered = scatter(ray, hit, material, vec3<f32>(u_bsdf, u_scalar.x), u_secondary);
        if !scattered.valid {
            break;
        }

        if use_nee && scattered.non_delta {
            // At the last allowed interaction there is no BSDF-sampled
            // continuation to share the light with.
            let last_vertex = depth + 1u >= params.max_depth;
            let wo = -normalize(ray.direction);
            let u_light = sample_2d(slot + 1u);
            if !have_scalar {
                u_scalar = sample_2d(slot + 2u);
                have_scalar = true;
            }
            color = color + throughput * sample_direct_light(hit, material, wo, last_vertex, u_light, u_scalar.x);
            prev_specular = false;
            prev_pdf = scattered.pdf;
        } else {
            prev_specular = true;
        }

        throughput = throughput * scattered.attenuation;
        ray = scattered.ray;

        // Russian roulette after depth 5
        if depth > 5u {
            let p = max(throughput.r, max(throughput.g, throughput.b));
            if !have_scalar {
                u_scalar = sample_2d(slot + 2u);
                have_scalar = true;
            }
            if u_scalar.y > p {
                break;
            }
            throughput = throughput / p;
        }
    }

    return color;
}

// ============================================================================
// Main Compute Kernel
// ============================================================================

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let pixel = global_id.xy;

    // Bounds check
    if pixel.x >= params.width || pixel.y >= params.height {
        return;
    }

    sampler_pixel_seed = hash(pixel.y * params.width + pixel.x);

    var color = vec3<f32>(0.0);

    // Render samples for this pass, continuing each pixel's sample sequence
    let samples_this_pass = params.samples;
    for (var s: u32 = 0u; s < samples_this_pass; s = s + 1u) {
        sampler_index = params.sample_offset + s;
        let ray = generate_ray(pixel);
        color = color + trace_path(ray);
    }

    let idx = pixel.y * params.width + pixel.x;

    // Accumulate with previous passes (frame_seed > 0 means not first pass)
    if params.frame_seed > 0u {
        let prev = output[idx];
        color = color + prev.xyz;
    }

    // Store accumulated color (no gamma, no division - done in final readback)
    output[idx] = vec4<f32>(color, 0.0);
}
