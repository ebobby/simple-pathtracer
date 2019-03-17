use super::{Scatterable, Scattered};
use crate::intersectable::Intersection;
use crate::ray::Ray;
use crate::Color;
use crate::Texture;
use crate::Vec3;

#[derive(Clone, Debug)]
pub struct DiffuseLight {
    pub texture: Texture,
}

impl Scatterable for DiffuseLight {
    fn emit(&self, u: f64, v: f64, p: Vec3) -> Color {
        self.texture.value(u, v, p)
    }

    fn scatter(&self, _ray: &Ray, _intersection: &Intersection) -> Option<Scattered> {
        None
    }
}
