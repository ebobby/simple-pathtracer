use pathtracer::shape::*;
use pathtracer::Camera;
use pathtracer::Color;
use pathtracer::Hitable;
use pathtracer::Material;
use pathtracer::Scene;
use pathtracer::Texture;
use pathtracer::Vec3;
use pathtracer::BVH;

fn cornell_box(aspect_ratio: f64) -> Scene {
    let red = Color::new(0.75, 0.25, 0.25);
    let white = Color::new(0.75, 0.75, 0.75);
    let blue = Color::new(0.25, 0.25, 0.75);
    let light = Color::new(1.0, 1.0, 1.0) * 15.0;


    let objects: Vec<Hitable> = vec![
        // light
        Box::new(Disc {
            center: Vec3::new(0.0, 0.0, -5.0),
            radius: 1.5,
            normal: Vec3::new(0.0, 1.0, 0.0),
            material: Material::diffuse_light(light),
        }),
        // right wall
        Box::new(Sphere {
            center: Vec3::new(5006.0, 0.0, 0.0),
            radius: 5000.0,
            material: Material::lambertian(Texture::constant_color(blue)),
        }),
        // left wall
        Box::new(Sphere {
            center: Vec3::new(-5006.0, 0.0, 0.0),
            radius: 5000.0,
            material: Material::lambertian(Texture::constant_color(red)),
        }),
        // ceiling
        Box::new(Sphere {
            center: Vec3::new(0.0, 5010.0, 0.0),
            radius: 5000.0,
            material: Material::lambertian(Texture::constant_color(white)),
        }),
        // floor
        Box::new(Sphere {
            center: Vec3::new(0.0, -5000.0, 0.0),
            radius: 5000.0,
            material: Material::lambertian(Texture::constant_color(white)),
        }),
        // back wall
        Box::new(Sphere {
            center: Vec3::new(0.0, 0.0, -5010.0),
            radius: 5000.0,
            material: Material::lambertian(Texture::constant_color(white)),
        }),
        Box::new(Sphere {
            center: Vec3::new(-3.5, 2.0, -3.0),
            radius: 2.0,
            material: Material::dielectric(Color::new(1.0, 1.0, 1.0), 1.52),
        }),
        Box::new(Sphere {
            center: Vec3::new(3.5, 2.0, -7.0),
            radius: 2.0,
            material: Material::metal(Color::new(0.05, 1.0, 0.05), 0.25),
        }),
        Box::new(Sphere {
            center: Vec3::new(3.8, 2.0, -1.5),
            radius: 2.0,
            material: Material::metal(Color::new(1.0, 0.05, 0.05), 0.0),
        }),
    ];

    let look_from = Vec3::new(0.0, 9.95, 8.0);
    let look_at = Vec3::new(0.0, 3.0, -5.0);

    Scene {
        camera: Camera::new(look_from, look_at, 55.0, aspect_ratio, 0.0),
        world: BVH::from_vec(objects),
    }
}

fn main() {
    let width = 640;
    let height = 480;
    let samples = 2500;
    let aspect_ratio = f64::from(width) / f64::from(height);
    let gamma = 2.2f64;
    let max_depth = 100;
    let workers: usize = 12;

    let scene = cornell_box(aspect_ratio);

    pathtracer::render(
        scene,
        width,
        height,
        samples,
        max_depth,
        gamma,
        workers,
        "output/inverted-light-cornell.png",
    );
}
