//! Bounding volume hierarchy stored as a flat array of nodes.

use crate::aabb::AABB;
use crate::intersectable::{Intersectable, Intersection};
use crate::light::Light;
use crate::ray::Ray;
use crate::{Hitable, Vec3};

/// One node of the flattened tree. Interior nodes have their left child at
/// `index + 1` (depth-first layout) and their right child at `child_or_shape`.
/// Leaves reference a single shape at `child_or_shape`.
#[derive(Debug, Clone, Copy)]
struct Node {
    bounding_box: AABB,
    child_or_shape: u32,
    is_leaf: bool,
}

#[derive(Debug)]
pub struct BVH {
    nodes: Vec<Node>,
    shapes: Vec<Hitable>,
    lights: Vec<Light>,
    /// Cumulative selection probabilities, parallel to `lights`.
    light_cdf: Vec<f64>,
    /// Index into `lights` for each shape, or `usize::MAX`.
    light_of_shape: Vec<usize>,
}

impl BVH {
    pub fn from_vec(objects: Vec<Hitable>) -> Self {
        if objects.is_empty() {
            panic!("I need a non-empty object list!");
        }

        let boxes: Vec<AABB> = objects.iter().map(|o| o.bounding_box()).collect();
        let mut order: Vec<u32> = (0..objects.len() as u32).collect();
        let mut nodes = Vec::with_capacity(2 * objects.len() - 1);

        Self::build(&boxes, &mut order, &mut nodes);

        // Collect lights, weighted by emitted power (luminance x area).
        let mut lights = Vec::new();
        let mut powers = Vec::new();
        let mut light_of_shape = vec![usize::MAX; objects.len()];
        for (shape_id, shape) in objects.iter().enumerate() {
            if let Some((light_shape, emission)) = shape.as_light() {
                light_of_shape[shape_id] = lights.len();
                let luminance = 0.2126 * emission.r + 0.7152 * emission.g + 0.0722 * emission.b;
                powers.push((luminance * light_shape.area()).max(0.0));
                lights.push(Light {
                    shape_id,
                    shape: light_shape,
                    select_pdf: 0.0,
                });
            }
        }
        let total: f64 = powers.iter().sum();
        let mut light_cdf = Vec::with_capacity(lights.len());
        let mut cumulative = 0.0;
        for (light, power) in lights.iter_mut().zip(&powers) {
            light.select_pdf = if total > 0.0 {
                power / total
            } else {
                1.0 / powers.len() as f64
            };
            cumulative += light.select_pdf;
            light_cdf.push(cumulative);
        }

        Self {
            nodes,
            shapes: objects,
            lights,
            light_cdf,
            light_of_shape,
        }
    }

    /// All emissive shapes in the scene.
    pub fn lights(&self) -> &[Light] {
        &self.lights
    }

    /// Pick a light with probability proportional to its power, from a
    /// uniform `u` in [0, 1). Returns the light and its selection pdf.
    pub fn pick_light(&self, u: f64) -> (&Light, f64) {
        let index = self
            .light_cdf
            .partition_point(|&cdf| cdf <= u)
            .min(self.lights.len() - 1);
        let light = &self.lights[index];
        (light, light.select_pdf)
    }

    /// The light corresponding to a shape id from an [`Intersection`], if any.
    pub fn light_of_shape(&self, shape_id: usize) -> Option<&Light> {
        self.lights.get(*self.light_of_shape.get(shape_id)?)
    }

    /// Append the subtree for `order` (a set of shape indices) to `nodes`,
    /// splitting on the longest axis at the median.
    fn build(boxes: &[AABB], order: &mut [u32], nodes: &mut Vec<Node>) {
        let bounding_box = order
            .iter()
            .map(|&i| boxes[i as usize])
            .reduce(AABB::surrounding)
            .unwrap();

        if order.len() == 1 {
            nodes.push(Node {
                bounding_box,
                child_or_shape: order[0],
                is_leaf: true,
            });
            return;
        }

        let extent = bounding_box.max - bounding_box.min;
        let axis = if extent.x >= extent.y && extent.x >= extent.z {
            0
        } else if extent.y >= extent.z {
            1
        } else {
            2
        };
        let key = |i: &u32| -> f64 {
            let c = boxes[*i as usize].centroid();
            match axis {
                0 => c.x,
                1 => c.y,
                _ => c.z,
            }
        };
        let mid = order.len() / 2;
        order.select_nth_unstable_by(mid, |a, b| key(a).partial_cmp(&key(b)).unwrap());

        let (left, right) = order.split_at_mut(mid);

        let this = nodes.len();
        nodes.push(Node {
            bounding_box,
            child_or_shape: u32::MAX, // patched after the left subtree is built
            is_leaf: false,
        });

        Self::build(boxes, left, nodes);
        nodes[this].child_or_shape = nodes.len() as u32;
        Self::build(boxes, right, nodes);
    }
}

impl Intersectable for BVH {
    fn bounding_box(&self) -> AABB {
        self.nodes[0].bounding_box
    }

    fn intersect(&self, ray: &Ray, min: f64, max: f64) -> Option<Intersection<'_>> {
        let origin = ray.origin;
        let inv_dir = Vec3::new(
            ray.direction.x.recip(),
            ray.direction.y.recip(),
            ray.direction.z.recip(),
        );

        let mut closest: Option<Intersection<'_>> = None;
        let mut closest_t = max;

        // Explicit stack of node indices; each node's box is tested when popped.
        let mut stack = [0u32; 64];
        let mut stack_len = 1usize;

        while stack_len > 0 {
            stack_len -= 1;
            let index = stack[stack_len];
            let node = &self.nodes[index as usize];

            if node
                .bounding_box
                .intersect(origin, inv_dir, min, closest_t)
                .is_none()
            {
                continue;
            }

            if node.is_leaf {
                let shape_id = node.child_or_shape as usize;
                if let Some(mut hit) = self.shapes[shape_id].intersect(ray, min, closest_t) {
                    hit.shape_id = shape_id;
                    closest_t = hit.t;
                    closest = Some(hit);
                }
            } else {
                stack[stack_len] = index + 1;
                stack[stack_len + 1] = node.child_or_shape;
                stack_len += 2;
            }
        }

        closest
    }
}
