use crate::Color;
use crate::Vec3;

mod bitmap;
mod checker;
mod constant_color;

use bitmap::Bitmap;
use checker::Checker;
use constant_color::ConstantColor;

/// Texture object.
#[derive(Clone, Debug)]
pub enum Texture {
    Bitmap(Bitmap),
    Checker(Checker),
    ConstantColor(ConstantColor),
}

pub trait TextureObject {
    fn value(&self, u: f64, v: f64, p: Vec3) -> Color;
}

impl Texture {
    pub fn bitmap(path: &str) -> Texture {
        Texture::Bitmap(Bitmap {
            bitmap: image::open(path).unwrap(),
        })
    }

    pub fn checker(squares: usize, odd: Color, even: Color) -> Texture {
        Texture::Checker(Checker { squares, odd, even })
    }

    pub fn constant_color(color: Color) -> Texture {
        Texture::ConstantColor(ConstantColor { color })
    }

    pub fn value(&self, u: f64, v: f64, p: Vec3) -> Color {
        match self {
            Texture::Bitmap(bitmap) => bitmap.value(u, v, p),
            Texture::Checker(checker) => checker.value(u, v, p),
            Texture::ConstantColor(color) => color.value(u, v, p),
        }
    }
}
