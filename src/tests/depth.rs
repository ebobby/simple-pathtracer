//! `radiance` must honor the caller's `max_depth`.

use crate::radiance;
use crate::ray::Ray;
use crate::shape::Sphere;
use crate::{Camera, Color, Hitable, Material, Scene, Texture, Vec3, BVH};

/// A perfect mirror in front of the camera, and a light behind it.
/// A ray towards the mirror needs two surface interactions to see the light.
fn mirror_facing_light() -> Scene {
    let mirror = Sphere {
        center: Vec3::new(0.0, 0.0, -1005.0),
        radius: 1000.0,
        material: Material::metal(Texture::constant_color(Color::new(1.0, 1.0, 1.0)), 0.0),
    };
    let light = Sphere {
        center: Vec3::new(0.0, 0.0, 1005.0),
        radius: 1000.0,
        material: Material::diffuse_light(Texture::constant_color(Color::new(2.0, 3.0, 4.0))),
    };
    let objects: Vec<Hitable> = vec![Box::new(mirror), Box::new(light)];
    Scene {
        camera: Camera::new(Vec3::zero(), Vec3::new(0.0, 0.0, -1.0), 45.0, 1.0, 0.0),
        world: BVH::from_vec(objects),
    }
}

fn towards_mirror() -> Ray {
    Ray {
        origin: Vec3::zero(),
        direction: Vec3::new(0.0, 0.0, -1.0),
    }
}

#[test]
fn max_depth_one_stops_at_first_surface() {
    let scene = mirror_facing_light();
    let c = radiance(&scene, &towards_mirror(), 1, 1);
    assert_eq!((c.r, c.g, c.b), (0.0, 0.0, 0.0));
}

#[test]
fn max_depth_two_reaches_light_through_mirror() {
    let scene = mirror_facing_light();
    let c = radiance(&scene, &towards_mirror(), 1, 2);
    assert!((c.r - 2.0).abs() < 1e-9 && (c.g - 3.0).abs() < 1e-9 && (c.b - 4.0).abs() < 1e-9, "{c:?}");
}
