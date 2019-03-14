use super::bvh::BVH;
use super::camera::Camera;

#[derive(Debug)]
pub struct Scene {
    pub camera: Camera,
    pub world: BVH,
}
