use super::{Scatterable, Scattered};
use crate::color::Color;
use crate::intersectable::Intersection;
use crate::ray::Ray;

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
