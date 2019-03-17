use super::TextureObject;
use crate::Color;
use crate::Vec3;

#[derive(Clone, Debug)]
pub struct Checker {
    pub squares: usize,
    pub even: Color,
    pub odd: Color,
}

impl TextureObject for Checker {
    fn value(&self, u: f64, v: f64, _p: Vec3) -> Color {
        let x = (u * self.squares as f64) as u32;
        let y = (v * self.squares as f64) as u32;

        match (is_even(x), is_even(y)) {
            (true, true) => self.even,
            (false, false) => self.even,
            _ => self.odd,
        }
    }
}

fn is_even(v: u32) -> bool {
    v % 2 == 0
}
