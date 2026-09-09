//! GPU scene builder - converts CPU scene to GPU-friendly format.

use crate::aabb::AABB;
use crate::environment::luminance;
use crate::gpu_types::*;
use crate::shape::{Disc, Sphere};
use crate::{Camera, Environment, Material, Vec3};

use std::f64::consts::PI;

use std::collections::HashMap;

/// A shape that can be added to the GPU scene.
pub enum GPUShape {
    Sphere(Sphere),
    Disc(Disc),
}

impl GPUShape {
    fn bounding_box(&self) -> AABB {
        match self {
            GPUShape::Sphere(s) => {
                let corner = Vec3::new(s.radius, s.radius, s.radius);
                AABB {
                    min: s.center - corner,
                    max: s.center + corner,
                }
            }
            GPUShape::Disc(d) => {
                let corner = Vec3::new(d.radius, d.radius, d.radius);
                AABB {
                    min: d.center - corner,
                    max: d.center + corner,
                }
            }
        }
    }

    fn material(&self) -> &Material {
        match self {
            GPUShape::Sphere(s) => &s.material,
            GPUShape::Disc(d) => &d.material,
        }
    }
}

/// Intermediate BVH node for construction.
struct BVHBuildNode {
    aabb: AABB,
    left: Option<Box<BVHBuildNode>>,
    right: Option<Box<BVHBuildNode>>,
    shape_idx: Option<usize>,
}

/// GPU-ready scene data.
pub struct GPUScene {
    pub camera: GPUCamera,
    pub bvh_nodes: Vec<GPUBVHNode>,
    pub spheres: Vec<GPUSphere>,
    pub discs: Vec<GPUDisc>,
    pub materials: Vec<GPUMaterial>,
    /// Emissive shapes (and the sky and sun, with sentinel indices) with
    /// power-proportional selection probabilities.
    pub lights: Vec<GPULight>,
    pub environment: GPUEnvironment,
    pub num_spheres: u32,
    pub num_discs: u32,
}

impl GPUScene {
    /// Build a GPU scene from shapes and camera, with no environment.
    pub fn build(shapes: Vec<GPUShape>, camera: &Camera) -> Self {
        Self::build_with_environment(shapes, camera, &Environment::default())
    }

    /// Build a GPU scene from shapes, camera and environment.
    pub fn build_with_environment(
        shapes: Vec<GPUShape>,
        camera: &Camera,
        environment: &Environment,
    ) -> Self {
        let mut spheres = Vec::new();
        let mut discs = Vec::new();
        let mut materials = Vec::new();
        let mut material_cache: HashMap<String, u32> = HashMap::new();

        // Helper to get or create material index
        let mut get_material_idx = |material: &Material| -> u32 {
            let key = format!("{:?}", material);
            if let Some(&idx) = material_cache.get(&key) {
                idx
            } else {
                let idx = materials.len() as u32;
                materials.push(GPUMaterial::from(material));
                material_cache.insert(key, idx);
                idx
            }
        };

        // Shape indices for BVH (spheres first, then discs)
        let mut shape_indices: Vec<(usize, AABB)> = Vec::new();

        // Extract spheres and discs
        for shape in &shapes {
            match shape {
                GPUShape::Sphere(s) => {
                    let material_idx = get_material_idx(&s.material);
                    let idx = spheres.len();
                    spheres.push(GPUSphere::new(
                        s.center.into(),
                        s.radius as f32,
                        material_idx,
                    ));
                    shape_indices.push((idx, shape.bounding_box()));
                }
                GPUShape::Disc(d) => {
                    let material_idx = get_material_idx(&d.material);
                    let idx = spheres.len() + discs.len(); // Offset by sphere count
                    discs.push(GPUDisc::new(
                        d.center.into(),
                        d.normal.into(),
                        d.radius as f32,
                        material_idx,
                    ));
                    shape_indices.push((idx, shape.bounding_box()));
                }
            }
        }

        let num_spheres = spheres.len() as u32;
        let num_discs = discs.len() as u32;

        // Lights, weighted by emitted power, matching `Scene::build_lights`:
        // shapes π L A, infinite lights irradiance x projected scene area.
        let mut light_shapes = Vec::new();
        let mut powers = Vec::new();
        for (i, shape) in shapes.iter().enumerate() {
            if shape.material().is_emissive() {
                let (center, area) = match shape {
                    GPUShape::Sphere(s) => (s.center, 4.0 * PI * s.radius * s.radius),
                    GPUShape::Disc(d) => (d.center, PI * d.radius * d.radius),
                };
                let emission = shape.material().emit(0.5, 0.5, center);
                light_shapes.push(i as u32);
                powers.push((PI * luminance(emission) * area).max(0.0));
            }
        }
        let bounds = shape_indices
            .iter()
            .map(|(_, b)| *b)
            .reduce(AABB::surrounding);
        let radius = bounds.map_or(1.0, |b| (b.max - b.min).length() * 0.5);
        let projected_area = PI * radius * radius;
        if let Some(sky) = &environment.sky {
            light_shapes.push(LIGHT_SKY);
            powers.push((sky.irradiance() * projected_area).max(0.0));
        }
        if let Some(sun) = &environment.sun {
            light_shapes.push(LIGHT_SUN);
            powers.push((sun.irradiance() * projected_area).max(0.0));
        }
        let total: f64 = powers.iter().sum();
        let mut cumulative = 0.0;
        let lights: Vec<GPULight> = light_shapes
            .iter()
            .zip(&powers)
            .map(|(&shape_idx, &power)| {
                let pdf = if total > 0.0 { power / total } else { 1.0 / powers.len() as f64 };
                cumulative += pdf;
                GPULight::new(shape_idx, pdf as f32, cumulative as f32)
            })
            .collect();

        // Build BVH
        let bvh_nodes = if shape_indices.is_empty() {
            // Empty scene - create a dummy node
            vec![GPUBVHNode::leaf(GPUVec3::zero(), GPUVec3::zero(), 0)]
        } else {
            let len = shape_indices.len();
            let root = Self::build_bvh(&mut shape_indices, 0, len);
            Self::flatten_bvh(&root)
        };

        Self {
            camera: GPUCamera::from(camera),
            bvh_nodes,
            spheres,
            discs,
            materials,
            lights,
            environment: GPUEnvironment::from(environment),
            num_spheres,
            num_discs,
        }
    }

    /// Build BVH recursively.
    fn build_bvh(shapes: &mut [(usize, AABB)], start: usize, end: usize) -> BVHBuildNode {
        let count = end - start;

        if count == 1 {
            let (shape_idx, aabb) = shapes[start];
            return BVHBuildNode {
                aabb,
                left: None,
                right: None,
                shape_idx: Some(shape_idx),
            };
        }

        // Find combined bounding box
        let mut combined = shapes[start].1;
        for i in (start + 1)..end {
            combined = AABB::surrounding(combined, shapes[i].1);
        }

        // Choose axis with largest extent
        let extent = combined.max - combined.min;
        let axis = if extent.x > extent.y && extent.x > extent.z {
            0
        } else if extent.y > extent.z {
            1
        } else {
            2
        };

        // Sort by axis
        shapes[start..end].sort_by(|a, b| {
            let a_center = (a.1.min.x + a.1.max.x) * 0.5;
            let b_center = (b.1.min.x + b.1.max.x) * 0.5;
            let (a_val, b_val) = match axis {
                0 => (a_center, b_center),
                1 => {
                    let ac = (a.1.min.y + a.1.max.y) * 0.5;
                    let bc = (b.1.min.y + b.1.max.y) * 0.5;
                    (ac, bc)
                }
                _ => {
                    let ac = (a.1.min.z + a.1.max.z) * 0.5;
                    let bc = (b.1.min.z + b.1.max.z) * 0.5;
                    (ac, bc)
                }
            };
            a_val.partial_cmp(&b_val).unwrap()
        });

        let mid = start + count / 2;

        if count == 2 {
            let (left_idx, left_aabb) = shapes[start];
            let (right_idx, right_aabb) = shapes[start + 1];

            BVHBuildNode {
                aabb: AABB::surrounding(left_aabb, right_aabb),
                left: Some(Box::new(BVHBuildNode {
                    aabb: left_aabb,
                    left: None,
                    right: None,
                    shape_idx: Some(left_idx),
                })),
                right: Some(Box::new(BVHBuildNode {
                    aabb: right_aabb,
                    left: None,
                    right: None,
                    shape_idx: Some(right_idx),
                })),
                shape_idx: None,
            }
        } else {
            let left = Self::build_bvh(shapes, start, mid);
            let right = Self::build_bvh(shapes, mid, end);

            BVHBuildNode {
                aabb: AABB::surrounding(left.aabb, right.aabb),
                left: Some(Box::new(left)),
                right: Some(Box::new(right)),
                shape_idx: None,
            }
        }
    }

    /// Flatten BVH tree to array using DFS order.
    fn flatten_bvh(root: &BVHBuildNode) -> Vec<GPUBVHNode> {
        let mut nodes = Vec::new();
        let mut node_indices: HashMap<*const BVHBuildNode, u32> = HashMap::new();

        // First pass: assign indices
        let mut idx = 0u32;
        let mut to_process: Vec<&BVHBuildNode> = vec![root];
        while let Some(node) = to_process.pop() {
            node_indices.insert(node as *const _, idx);
            idx += 1;

            if let Some(ref right) = node.right {
                to_process.push(right);
            }
            if let Some(ref left) = node.left {
                to_process.push(left);
            }
        }

        // Second pass: build flat array
        let mut to_process: Vec<&BVHBuildNode> = vec![root];
        while let Some(node) = to_process.pop() {
            let aabb_min = GPUVec3::new(
                node.aabb.min.x as f32,
                node.aabb.min.y as f32,
                node.aabb.min.z as f32,
            );
            let aabb_max = GPUVec3::new(
                node.aabb.max.x as f32,
                node.aabb.max.y as f32,
                node.aabb.max.z as f32,
            );

            let gpu_node = if let Some(shape_idx) = node.shape_idx {
                // Leaf node
                GPUBVHNode::leaf(aabb_min, aabb_max, shape_idx as u32)
            } else {
                // Interior node
                let left_idx = node
                    .left
                    .as_ref()
                    .map(|n| *node_indices.get(&(n.as_ref() as *const _)).unwrap())
                    .unwrap_or(u32::MAX);
                let right_idx = node
                    .right
                    .as_ref()
                    .map(|n| *node_indices.get(&(n.as_ref() as *const _)).unwrap())
                    .unwrap_or(u32::MAX);

                GPUBVHNode::interior(aabb_min, aabb_max, left_idx, right_idx)
            };

            nodes.push(gpu_node);

            if let Some(ref right) = node.right {
                to_process.push(right);
            }
            if let Some(ref left) = node.left {
                to_process.push(left);
            }
        }

        nodes
    }
}
