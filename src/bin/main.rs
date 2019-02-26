use pathtracer::camera::Camera;
use pathtracer::color::Color;
use pathtracer::intersectable::reversed_normal::ReversedNormal;
use pathtracer::intersectable::sphere::Sphere;
use pathtracer::intersectable::IntersectableList;
use pathtracer::material::Material;
use pathtracer::scene::Scene;
use pathtracer::vector::Vec3;

fn main() {
    let width = 1920;
    let height = 1080;
    let samples = 5000;
    let aspect_ratio = f64::from(width) / f64::from(height);
    let gamma = 2.2f64;

    let mut world = IntersectableList::new();

    world.push(Box::new(Sphere {
        position: Vec3::new(0., -1000., 0.),
        radius: 1000.,
        material: Material::Lambertian(Color::new(0.5, 0.5, 0.5)),
    }));
    world.push(Box::new(ReversedNormal {
        intersectable: Box::new(Sphere {
            position: Vec3::new(0., 0., 0.),
            radius: 50.,
            material: Material::Lambertian(Color::new(0.2, 0.2, 0.2)),
        }),
    }));
    world.push(Box::new(Sphere {
        position: Vec3::new(0., 3., 0.),
        radius: 3.0,
        material: Material::Lambertian(Color::new(0.15, 0.15, 0.15)),
    }));
    world.push(Box::new(Sphere {
        position: Vec3::new(-6., 3., 1.),
        radius: 3.0,
        material: Material::Metal(Color::new(0.75, 0.3, 0.3), 0.0),
    }));
    world.push(Box::new(Sphere {
        position: Vec3::new(6., 3., 1.),
        radius: 3.0,
        material: Material::Lambertian(Color::from_u8(0xEE, 0xE8, 0xAA)),
    }));
    world.push(Box::new(Sphere {
        position: Vec3::new(0., 30., 0.),
        radius: 18.0,
        material: Material::DiffuseLight(Color::new(1.0, 1.0, 1.0) * 2.5),
    }));
//    world.push(Box::new(Sphere {
//        position: Vec3::new(0., 1., 6.),
//        radius: 1.0,
//        material: Material::DiffuseLight(Color::from_u8(255, 160, 122) * 4.),
//    }));
//    world.push(Box::new(Sphere {
//        position: Vec3::new(-6., 1., 6.),
//        radius: 1.0,
//        material: Material::DiffuseLight(Color::from_u8(128, 128, 0) * 2.),
//    }));
//    world.push(Box::new(Sphere {
//        position: Vec3::new(6., 1., 6.),
//        radius: 1.0,
//        material: Material::DiffuseLight(Color::from_u8(100,149,237) * 2.),
//    }));

    let scene = Scene {
        camera: Camera::new(
            Vec3::new(15., 1., 10.),
            Vec3::new(0., 0., 0.),
            55.,
            aspect_ratio,
            0.,
        ),
        objects: world,
    };

    pathtracer::render(scene, width, height, samples, gamma, 12, "result.png");
}
