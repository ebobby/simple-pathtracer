use super::camera::Camera;
use super::intersectable::IntersectableList;

#[derive(Debug)]
pub struct Scene {
    pub camera: Camera,
    pub objects: IntersectableList,
}
