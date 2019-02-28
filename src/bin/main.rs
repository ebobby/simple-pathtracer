use pathtracer::camera::Camera;
use pathtracer::color::Color;
use pathtracer::intersectable::reversed_normal::ReversedNormal;
use pathtracer::intersectable::sphere::Sphere;
use pathtracer::intersectable::IntersectableList;
use pathtracer::material::Material;
use pathtracer::scene::Scene;
use pathtracer::vector::Vec3;

fn random_scene() -> IntersectableList {
    let mut list = IntersectableList::new();

    list.push(Box::new(Sphere {
        center: Vec3::new(0.0, -5000.5, -1.0),
        radius: 5000.0,
        material: Material::Lambertian(Color::new(0.5, 0.5, 0.5)),
    }));
    list.push(Box::new(Sphere {
        center: Vec3::new(0.0, 0.0, -1.0),
        radius: 0.5,
        material: Material::Lambertian(Color::new(0.8, 0.3, 0.3)),
    }));
    list.push(Box::new(Sphere {
        center: Vec3::new(1.0, 0.0, -1.0),
        radius: 0.5,
        material: Material::Metal(Color::new(0.8, 0.6, 0.2), 0.0),
    }));
    list.push(Box::new(Sphere {
        center: Vec3::new(-1.0, 0.0, -1.0),
        radius: 0.5,
        material: Material::Metal(Color::new(0.8, 0.8, 0.8), 0.0),
    }));

    list
}

fn main() {
    let width = 640;
    let height = 480;
    let samples = 50;
    let aspect_ratio = f64::from(width) / f64::from(height);
    let gamma = 2.2f64;

    let world = random_scene();

    let scene = Scene {
        camera: Camera::new(
            //            Vec3::new(-18.0, 2.0, 15.0),
            Vec3::new(1.0, 1.0, 1.5),
            Vec3::new(0.0, 0.0, -1.0),
            55.0,
            aspect_ratio,
            0.,
        ),
        objects: world,
    };

    pathtracer::render(scene, width, height, samples, gamma, 12, "result.png");
}
