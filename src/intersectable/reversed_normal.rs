use super::Intersectable;
use crate::material::Material;
use crate::ray::Ray;
use crate::vector::Vec3;

#[derive(Debug)]
pub struct ReversedNormal {
    pub intersectable: Box<dyn Intersectable + Send>,
}

impl Intersectable for ReversedNormal {
    fn intersect(&self, ray: Ray, min: f64, max: f64) -> Option<f64> {
        self.intersectable.intersect(ray, min, max)
    }

    fn material(&self) -> Material {
        self.intersectable.material()
    }

    fn normal(&self, point: Vec3) -> Vec3 {
        -self.intersectable.normal(point)
    }
}
