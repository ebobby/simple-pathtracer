use super::TextureObject;
use crate::Color;
use crate::Vec3;

#[derive(Clone, Debug)]
pub struct ConstantColor {
    pub color: Color,
}

impl TextureObject for ConstantColor {
    fn value(&self, _u: f64, _v: f64, _p: Vec3) -> Color {
        self.color
    }
}
