use crate::Color;
use crate::Vec3;

mod constant_color;

use constant_color::ConstantColor;

/// Texture object.
#[derive(Clone, Copy, Debug)]
pub enum Texture {
    ConstantColor(ConstantColor),
}

pub trait TextureObject {
    fn value(&self, u: f64, v: f64, p: Vec3) -> Color;
}

impl Texture  {
    pub fn constant_color(color: Color) -> Texture {
        Texture::ConstantColor(ConstantColor { color })
    }

    pub fn value(&self, u: f64, v: f64, p: Vec3) -> Color {
        match *self {
            Texture::ConstantColor(color) => color.value(u, v, p),
        }
    }
}
