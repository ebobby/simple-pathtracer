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
}

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

struct Material {
    color: vec4<f32>,
    material_type: u32,
    fuzz: f32,
    ior: f32,
    _pad: f32,
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
    pdf: f32,      // solid-angle pdf for diffuse scattering, 0 for specular
    diffuse: bool,
    valid: bool,
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
@group(0) @binding(7) var<storage, read> lights: array<u32>; // shape indices

// ============================================================================
// Random Number Generation (PCG-based for speed)
// ============================================================================

var<private> rng_state: u32;

// Fast hash for seed initialization
fn hash(x: u32) -> u32 {
    var v = x;
    v = v ^ (v >> 16u);
    v = v * 0x7feb352du;
    v = v ^ (v >> 15u);
    v = v * 0x846ca68bu;
    v = v ^ (v >> 16u);
    return v;
}

fn init_rng(pixel: vec2<u32>, frame: u32) {
    // Create unique seed per pixel and frame using hash
    let seed = pixel.x + pixel.y * params.width + frame * params.width * params.height;
    rng_state = hash(seed);
    // Warm up the RNG
    rng_state = rng_state * 747796405u + 2891336453u;
    rng_state = rng_state * 747796405u + 2891336453u;
}

fn random() -> f32 {
    // PCG-XSH-RR: fast and good quality
    rng_state = rng_state * 747796405u + 2891336453u;
    let word = ((rng_state >> ((rng_state >> 28u) + 4u)) ^ rng_state) * 277803737u;
    let result = (word >> 22u) ^ word;
    return f32(result) / 4294967295.0;
}

fn random_in_unit_sphere() -> vec3<f32> {
    // Rejection sampling (kept for metal fuzz)
    loop {
        let x = 2.0 * random() - 1.0;
        let y = 2.0 * random() - 1.0;
        let z = 2.0 * random() - 1.0;
        let p = vec3<f32>(x, y, z);
        if dot(p, p) <= 1.0 {
            return p;
        }
    }
    return vec3<f32>(0.0, 0.0, 0.0); // unreachable
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

// Cosine-weighted hemisphere sampling
fn random_cosine_direction(normal: vec3<f32>) -> vec3<f32> {
    let r1 = random();
    let r2 = random();

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

fn tent_filter() -> f32 {
    let r = 2.0 * random();
    if r < 1.0 {
        return sqrt(r) - 1.0;
    } else {
        return 1.0 - sqrt(2.0 - r);
    }
}

// ============================================================================
// Ray Generation
// ============================================================================

fn generate_ray(pixel: vec2<u32>) -> Ray {
    // Simple random jitter within pixel
    let u = (f32(pixel.x) + random()) / f32(params.width);
    let v = (f32(pixel.y) + random()) / f32(params.height);

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

fn scatter(ray: Ray, hit: HitRecord, material: Material) -> ScatterResult {
    var result: ScatterResult;
    result.valid = false;
    result.diffuse = false;
    result.pdf = 0.0;

    // Offset origin along normal to prevent self-intersection
    let offset_origin = hit.p + hit.normal * 0.001;

    switch material.material_type {
        case MATERIAL_LAMBERTIAN: {
            // Cosine-weighted diffuse scattering (matches CPU implementation)
            let direction = random_cosine_direction(hit.normal);
            result.ray = Ray(offset_origin, direction);
            result.attenuation = material.color.xyz;
            result.pdf = max(dot(direction, hit.normal), 0.0) / PI;
            result.diffuse = true;
            result.valid = true;
        }
        case MATERIAL_METAL: {
            // Specular reflection with fuzz
            let reflected = reflect(normalize(ray.direction), hit.normal);
            let scattered_dir = reflected + material.fuzz * random_in_unit_sphere();
            if dot(scattered_dir, hit.normal) > 0.0 {
                result.ray = Ray(offset_origin, scattered_dir);
                result.attenuation = material.color.xyz;
                result.valid = true;
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
                cosine = material.ior * d / length(ray.direction);
            } else {
                // Ray entering glass (hitting from outside)
                outward_normal = hit.normal;
                ni_over_nt = 1.0 / material.ior;
                cosine = -d / length(ray.direction);
            }

            let cos_theta = min(dot(-unit_direction, outward_normal), 1.0);
            let sin_theta = sqrt(1.0 - cos_theta * cos_theta);
            let cannot_refract = ni_over_nt * sin_theta > 1.0;

            var direction: vec3<f32>;
            if cannot_refract || schlick(cosine, material.ior) > random() {
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
        default: {
            result.valid = false;
        }
    }

    return result;
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
fn light_sample(shape_idx: u32, p: vec3<f32>) -> LightSample {
    var s: LightSample;
    s.valid = false;

    let u1 = random();
    let u2 = random();
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

// Direct lighting at a Lambertian vertex from one uniformly chosen light,
// weighted against BSDF sampling unless full_weight is set.
fn sample_direct_light(hit: HitRecord, albedo: vec3<f32>, full_weight: bool) -> vec3<f32> {
    let light_index = min(u32(random() * f32(params.num_lights)), params.num_lights - 1u);
    let light_shape = lights[light_index];

    let ls = light_sample(light_shape, hit.p);
    if !ls.valid {
        return vec3<f32>(0.0);
    }
    let cos_theta = dot(ls.direction, hit.normal);
    if cos_theta <= 0.0 {
        return vec3<f32>(0.0);
    }

    let shadow_ray = Ray(hit.p + hit.normal * 0.001, ls.direction);
    let shadow_hit = intersect_bvh(shadow_ray);
    if !shadow_hit.valid || shadow_hit.shape_idx != light_shape {
        return vec3<f32>(0.0);
    }

    let emitted = materials[shadow_hit.material_idx].color.xyz;
    let pdf_light = ls.pdf / f32(params.num_lights);
    let pdf_bsdf = cos_theta / PI;
    var weight = 1.0;
    if !full_weight {
        weight = power_heuristic(pdf_light, pdf_bsdf);
    }

    // f * cos / pdf with f = albedo / PI
    return albedo * emitted * (cos_theta / (PI * pdf_light) * weight);
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
            // Miss - nothing outside the scene emits light (matches CPU)
            break;
        }

        let material = materials[hit.material_idx];

        // Add emission from lights
        if material.material_type == MATERIAL_DIFFUSE_LIGHT {
            var weight = 1.0;
            if use_nee && !prev_specular {
                let direction = normalize(ray.direction);
                let pdf_light = light_pdf(hit.shape_idx, ray.origin, hit.p, direction) / f32(params.num_lights);
                weight = power_heuristic(prev_pdf, pdf_light);
            }
            color = color + throughput * material.color.xyz * weight;
            break;
        }

        // Scatter ray
        let scattered = scatter(ray, hit, material);
        if !scattered.valid {
            break;
        }

        if use_nee && scattered.diffuse {
            // At the last allowed interaction there is no BSDF-sampled
            // continuation to share the light with.
            let last_vertex = depth + 1u >= params.max_depth;
            color = color + throughput * sample_direct_light(hit, scattered.attenuation, last_vertex);
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
            if random() > p {
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

    // Initialize RNG for this pixel (frame_seed varies per pass for different random sequences)
    init_rng(pixel, params.frame_seed);

    var color = vec3<f32>(0.0);

    // Render samples for this pass
    let samples_this_pass = params.samples;
    for (var s: u32 = 0u; s < samples_this_pass; s = s + 1u) {
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
