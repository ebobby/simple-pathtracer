use super::Camera;
use super::BVH;

#[derive(Debug)]
pub struct Scene {
    pub camera: Camera,
    pub world: BVH,
}
