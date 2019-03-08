#![allow(dead_code)]
#![allow(unused_imports)]

use pathtracer::camera::Camera;
use pathtracer::color::Color;
use pathtracer::intersectable::Disc;
use pathtracer::intersectable::IntersectableList;
use pathtracer::intersectable::Plane;
use pathtracer::intersectable::Sphere;
use pathtracer::material::Material;
use pathtracer::scene::Scene;
use pathtracer::vector::Vec3;

fn raytracing_one_weekend(aspect_ratio: f64) -> Scene {
    let mut list = IntersectableList::new();

    list.push(Box::new(Sphere {
        center: Vec3::new(0.0, -1000.0, 0.0),
        radius: 1000.0,
        material: Material::Lambertian(Color::new(0.5, 0.5, 0.5)),
    }));

    for a in -11..11 {
        for b in -11..11 {
            let choose_mat = rand::random::<f64>();

            let center = Vec3::new(
                f64::from(a) + 0.9 * rand::random::<f64>(),
                0.2,
                f64::from(b) + 0.9 * rand::random::<f64>(),
            );

            if (center - Vec3::new(4.0, 0.2, 0.0)).length() > 0.9 {
                if choose_mat < 0.8 {
                    list.push(Box::new(Sphere {
                        center,
                        radius: 0.2,
                        material: Material::Lambertian(Color::new(
                            rand::random::<f64>() * rand::random::<f64>(),
                            rand::random::<f64>() * rand::random::<f64>(),
                            rand::random::<f64>() * rand::random::<f64>(),
                        )),
                    }));
                } else if choose_mat < 0.95 {
                    list.push(Box::new(Sphere {
                        center,
                        radius: 0.2,
                        material: Material::Metal(
                            Color::new(
                                0.5 * (1.0 + rand::random::<f64>()),
                                0.5 * (1.0 + rand::random::<f64>()),
                                0.5 * (1.0 + rand::random::<f64>()),
                            ),
                            0.5 * rand::random::<f64>(),
                        ),
                    }));
                } else {
                    list.push(Box::new(Sphere {
                        center,
                        radius: 0.2,
                        material: Material::Dielectric(Color::white(), 1.5),
                    }));
                }
            }
        }
    }

    list.push(Box::new(Sphere {
        center: Vec3::new(0.0, 1.0, 0.0),
        radius: 1.0,
        material: Material::Dielectric(Color::white(), 1.5),
    }));
    list.push(Box::new(Sphere {
        center: Vec3::new(-4.0, 1.0, 0.0),
        radius: 1.0,
        material: Material::Lambertian(Color::new(0.4, 0.2, 0.1)),
    }));
    list.push(Box::new(Sphere {
        center: Vec3::new(4.0, 1.0, 0.0),
        radius: 1.0,
        material: Material::Metal(Color::new(0.7, 0.6, 0.5), 0.0),
    }));

    list.push(Box::new(Sphere {
        center: Vec3::new(0.0, 0.0, 0.0),
        radius: 5000.0,
        material: Material::DiffuseLight(Color::new(0.5, 0.5, 0.5)),
    }));

    let look_from = Vec3::new(13.0, 2.0, 3.0);
    let look_at = Vec3::new(0.0, 0.0, 0.0);

    Scene {
        camera: Camera::new(look_from, look_at, 20.0, aspect_ratio, 0.0),
        objects: list,
    }
}

fn cornell_box(aspect_ratio: f64) -> Scene {
    let mut list = IntersectableList::new();

    let red = Color::new(0.65, 0.05, 0.05);
    let white = Color::new(0.73, 0.73, 0.73);
    let green = Color::new(0.12, 0.45, 0.15);
    let blue = Color::new(0.20, 0.20, 0.65);
    let gold = Color::from_u8(255, 215, 0);
    let light = Color::white() * 15.0;

    // light
    list.push(Box::new(Disc {
        center: Vec3::new(0.0, 10.0, -5.0),
        normal: Vec3::new(0.0, -1.0, 0.0),
        radius: 1.5,
        material: Material::DiffuseLight(light),
    }));

    // right wall
    list.push(Box::new(Plane {
        point: Vec3::new(5.0, 0.0, 0.0),
        normal: Vec3::new(-1.0, 0.0, 0.0),
        material: Material::Lambertian(green),
    }));
    // left wall
    list.push(Box::new(Plane {
        point: Vec3::new(-5.0, 0.0, 0.0),
        normal: Vec3::new(1.0, 0.0, 0.0),
        material: Material::Lambertian(red),
    }));
    // ceiling
    list.push(Box::new(Plane {
        point: Vec3::new(0.0, 10.0, 0.0),
        normal: Vec3::new(0.0, -1.0, 0.0),
        material: Material::Lambertian(white),
    }));
    // floor
    list.push(Box::new(Plane {
        point: Vec3::new(0.0, 0.0, 0.0),
        normal: Vec3::new(0.0, 1.0, 0.0),
        material: Material::Lambertian(white),
    }));
    // back wall
    list.push(Box::new(Plane {
        point: Vec3::new(0.0, 0.0, -10.0),
        normal: Vec3::new(0.0, 0.0, 1.0),
        material: Material::Lambertian(white),
    }));
    // back wall (behind the camera)
    list.push(Box::new(Plane {
        point: Vec3::new(0.0, 0.0, 10.0),
        normal: Vec3::new(0.0, 0.0, -1.0),
        material: Material::Lambertian(white),
    }));

    list.push(Box::new(Sphere {
        center: Vec3::new(-2.5, 2.0, -3.0),
        radius: 2.0,
        material: Material::Dielectric(Color::white(), 2.42),
    }));
    list.push(Box::new(Sphere {
        center: Vec3::new(2.5, 2.0, -7.0),
        radius: 2.0,
        material: Material::Metal(gold, 0.25),
    }));
    list.push(Box::new(Sphere {
        center: Vec3::new(3.8, 1.0, -2.5),
        radius: 1.0,
        material: Material::Lambertian(blue),
    }));

    let look_from = Vec3::new(0.0, 5.0, 10.0);
    let look_at = Vec3::new(0.0, 5.0, 0.0);

    Scene {
        camera: Camera::new(look_from, look_at, 50.0, aspect_ratio, 0.0),
        objects: list,
    }
}

fn test_scene(aspect_ratio: f64) -> Scene {
    let mut list = IntersectableList::new();

    let white = Color::new(0.53, 0.53, 0.53);

    let spotlight_intensity = 25.0;
    for x in -4..5 {
        let x = f64::from(x);

        list.push(Box::new(Disc {
            center: Vec3::new(x, 0.0, -2.0),
            normal: Vec3::new(0.0, 1.0, 0.0),
            radius: 0.25,
            material: Material::DiffuseLight(Color::new(1.0, 0.05, 0.05) * spotlight_intensity),
        }));
        list.push(Box::new(Disc {
            center: Vec3::new(x, 0.0, -4.0),
            normal: Vec3::new(0.0, 1.0, 0.0),
            radius: 0.25,
            material: Material::DiffuseLight(Color::new(0.05, 1.0, 0.05) * spotlight_intensity),
        }));
        list.push(Box::new(Disc {
            center: Vec3::new(x, 0.0, -6.0),
            normal: Vec3::new(0.0, 0.0, 1.0),
            radius: 0.25,
            material: Material::DiffuseLight(Color::new(0.05, 0.05, 1.0) * spotlight_intensity),
        }));
        list.push(Box::new(Disc {
            center: Vec3::new(x, 0.0, -8.0),
            normal: Vec3::new(0.0, 1.0, 1.0),
            radius: 0.25,
            material: Material::DiffuseLight(Color::new(1.0, 0.05, 1.0) * spotlight_intensity),
        }));
    }

    // right wall
    list.push(Box::new(Plane {
        point: Vec3::new(5.0, 0.0, 0.0),
        normal: Vec3::new(-1.0, 0.0, 0.0),
        material: Material::Lambertian(white),
    }));
    // left wall
    list.push(Box::new(Plane {
        point: Vec3::new(-5.0, 0.0, 0.0),
        normal: Vec3::new(1.0, 0.0, 0.0),
        material: Material::Lambertian(white),
    }));
    // ceiling
    list.push(Box::new(Plane {
        point: Vec3::new(0.0, 10.0, 0.0),
        normal: Vec3::new(0.0, -1.0, 0.0),
        material: Material::Lambertian(white),
    }));
    // floor
    list.push(Box::new(Plane {
        point: Vec3::new(0.0, 0.0, 0.0),
        normal: Vec3::new(0.0, 1.0, 0.0),
        material: Material::Lambertian(white),
    }));
    // back wall
    list.push(Box::new(Plane {
        point: Vec3::new(0.0, 0.0, -10.0),
        normal: Vec3::new(0.0, 0.0, 1.0),
        material: Material::Lambertian(white),
    }));
    // back wall (behind the camera)
    list.push(Box::new(Plane {
        point: Vec3::new(0.0, 0.0, 10.0),
        normal: Vec3::new(0.0, 0.0, -1.0),
        material: Material::Lambertian(white),
    }));

    list.push(Box::new(Sphere {
        center: Vec3::new(0.0, 5.0, -5.0),
        radius: 4.0,
        material: Material::Dielectric(Color::white(), 2.42),
    }));

    let look_from = Vec3::new(0.0, 5.0, 10.0);
    let look_at = Vec3::new(0.0, 5.0, 0.0);

    Scene {
        camera: Camera::new(look_from, look_at, 50.0, aspect_ratio, 0.0),
        objects: list,
    }
}

fn main() {
    let width = 1280;
    let height = 720;
    let samples = 10000;
    let aspect_ratio = f64::from(width) / f64::from(height);
    let gamma = 2.2f64;
    let max_depth = 50;
    let workers = 12;

    let scene = cornell_box(aspect_ratio);
    //let scene = raytracing_one_weekend(aspect_ratio);
    //let scene = test_scene(aspect_ratio);

    pathtracer::render(
        scene,
        width,
        height,
        samples,
        max_depth,
        gamma,
        workers,
        "result.png",
    );
}
