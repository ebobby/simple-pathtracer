use super::{Scatterable, Scattered};
use crate::intersectable::Intersection;
use crate::ray::Ray;
use crate::Color;

#[derive(Clone, Debug)]
pub struct DiffuseLight {
    pub color: Color,
}

impl Scatterable for DiffuseLight {
    fn emit(&self) -> Color {
        self.color
    }

    fn scatter(&self, _ray: &Ray, _intersection: &Intersection) -> Option<Scattered> {
        None
    }
}
