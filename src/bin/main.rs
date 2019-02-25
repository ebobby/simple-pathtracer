use pathtracer::camera::Camera;
use pathtracer::color::Color;
use pathtracer::intersectable::sphere::Sphere;
use pathtracer::intersectable::IntersectableList;
use pathtracer::material::Material;
use pathtracer::scene::Scene;
use pathtracer::vector::Vec3;

fn main() {
    let width = 1440;
    let height = 900;
    let samples = 500;
    let aspect_ratio = f64::from(width) / f64::from(height);

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
    world.push(Box::new(Sphere {
        position: Vec3::new(-6., 3., 0.5),
        radius: 3.0,
        material: Material::Lambertian(Color::new(0.5, 0.2, 0.5)),
    }));
    world.push(Box::new(Sphere {
        position: Vec3::new(6., 3., 0.5),
        radius: 3.0,
        material: Material::Lambertian(Color::new(0.5, 0.5, 0.2)),
    }));

    let scene = Scene {
        camera: Camera::new(
            Vec3::new(0., 5., 15.),
            Vec3::new(0., 0., 0.),
            60.,
            aspect_ratio,
            0.,
        ),
        objects: world,
    };

    pathtracer::render(scene, width, height, samples, 2.2, 12, "result.png");
}
