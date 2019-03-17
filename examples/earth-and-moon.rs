use pathtracer::shape::*;
use pathtracer::Camera;
use pathtracer::Color;
use pathtracer::Hitable;
use pathtracer::Material;
use pathtracer::Scene;
use pathtracer::Texture;
use pathtracer::Vec3;
use pathtracer::BVH;

fn earth_moon(aspect_ratio: f64) -> Scene {
    let objects: Vec<Hitable> = vec![
        Box::new(Disc {
            center: Vec3::new(1000.0, 0.0, 0.0),
            normal: Vec3::new(-1.0, 0.0, 0.0),
            radius: 1000.0,
            material: Material::diffuse_light(Color::new(1.0, 0.90, 0.75) * 5.0),
        }),
        Box::new(Sphere {
            center: Vec3::new(-9.0, 0.0, 0.0),
            radius: 10.0,
            material: Material::lambertian(Texture::bitmap("examples/textures/earth.jpg")),
        }),
        Box::new(Sphere {
            center: Vec3::new(13.0, 0.0, 0.0),
            radius: 5.0,
            material: Material::lambertian(Texture::bitmap("examples/textures/moon.jpg")),
        }),
    ];

    let look_from = Vec3::new(0.0, 20.0, 30.0);
    let look_at = Vec3::new(0.0, 0.0, 0.0);

    Scene {
        camera: Camera::new(look_from, look_at, 45.0, aspect_ratio, 0.0),
        world: BVH::from_vec(objects),
    }
}

fn main() {
    let width = 640;
    let height = 480;
    let samples = 5000;
    let aspect_ratio = f64::from(width) / f64::from(height);
    let gamma = 2.2f64;
    let max_depth = 50;
    let workers: usize = 8;

    let scene = earth_moon(aspect_ratio);

    pathtracer::render(
        scene,
        width,
        height,
        samples,
        max_depth,
        gamma,
        workers,
        "output/earth-moon.png",
    );
}
