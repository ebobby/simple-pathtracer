use super::vector::Vec3;

#[derive(Clone, Copy, Debug, Default)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}
