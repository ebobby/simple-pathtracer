use super::camera::Camera;
use super::intersectable::List;

#[derive(Debug)]
pub struct Scene {
    pub camera: Camera,
    pub objects: List,
}
