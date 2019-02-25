use pathtracer::camera::Camera;
use pathtracer::color::Color;
use pathtracer::intersectable::sphere::Sphere;
use pathtracer::intersectable::IntersectableList;
use pathtracer::material::Material;
use pathtracer::scene::Scene;
use pathtracer::vector::Vec3;

use std::time::Instant;

fn main() {
    let width = 1920;
    let height = 1080;
    let samples = 1000;
    let aspect_ratio = width as f64 / height as f64;

    let mut world = IntersectableList::new();

    world.push(Box::new(Sphere {
        position: Vec3::new(0., -1000., 0.),
        radius: 1000.,
        material: Material::Lambertian(Color::new(0.5, 0.5, 0.5)),
    }));
    world.push(Box::new(Sphere {
        position: Vec3::new(0., 3., 0.),
        radius: 3.0,
        material: Material::Lambertian(Color::new(0.2, 0.2, 0.2)),
    }));

    let scene = Scene {
        width,
        height,
        samples,
        camera: Camera::new(
            Vec3::new(0., 5., 15.),
            Vec3::new(0., 0., 0.),
            60.,
            aspect_ratio,
            0.,
        ),
        objects: world,
    };

    let now = Instant::now();

    scene.render("result.png".to_string());

    let duration = now.elapsed();

    println!(
        "{} milliseconds elapsed.",
        duration.as_secs() * 1000 + u64::from(duration.subsec_millis())
    );
}
