#![allow(dead_code)]
#![allow(unused_imports)]

use pathtracer::camera::Camera;
use pathtracer::intersectable::List;
use pathtracer::shape::*;
use pathtracer::Color;
use pathtracer::Hitable;
use pathtracer::Material;
use pathtracer::Scene;
use pathtracer::Vec3;

fn raytracing_one_weekend(aspect_ratio: f64) -> Scene {
    let mut list: Vec<Hitable> = Vec::new();
    let radius = 0.2f64;

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
                        radius,
                        material: Material::Lambertian(Color::new(
                            rand::random::<f64>() * rand::random::<f64>(),
                            rand::random::<f64>() * rand::random::<f64>(),
                            rand::random::<f64>() * rand::random::<f64>(),
                        )),
                    }));
                } else if choose_mat < 0.95 {
                    list.push(Box::new(Sphere {
                        center,
                        radius,
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
                        radius,
                        material: Material::Dielectric(Color::new(1.0, 1.0, 1.0), 1.5),
                    }));
                }
            }
        }
    }

    list.push(Box::new(Sphere {
        center: Vec3::new(0.0, 1.0, 0.0),
        radius: 1.0,
        material: Material::Dielectric(Color::new(1.0, 1.0, 1.0), 1.5),
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
        objects: List::from_vec(list),
    }
}

//fn cornell_box(aspect_ratio: f64) -> Scene {
//    let red = Color::new(0.65, 0.05, 0.05);
//    let white = Color::new(0.73, 0.73, 0.73);
//    let green = Color::new(0.12, 0.45, 0.15);
//    let blue = Color::new(0.20, 0.20, 0.65);
//    let gold = Color::from_u8(255, 215, 0);
//    let light = Color::new(1.0, 1.0, 1.0) * 15.0;
//
//    let objects: Vec<Hitable> = vec![
//        // light
//        Box::new(Disc::new(
//            Vec3::new(0.0, 10.0, -5.0),
//            1.5,
//            Vec3::new(0.0, -1.0, 0.0),
//            Material::DiffuseLight(light),
//        )),
//        // right wall
//        Box::new(Plane::new(
//            Vec3::new(5.0, 0.0, 0.0),
//            Vec3::new(-1.0, 0.0, 0.0),
//            Material::Lambertian(green),
//        )),
//        // left wall
//        Box::new(Plane::new(
//            Vec3::new(-5.0, 0.0, 0.0),
//            Vec3::new(1.0, 0.0, 0.0),
//            Material::Lambertian(red),
//        )),
//        // ceiling
//        Box::new(Plane::new(
//            Vec3::new(0.0, 10.0, 0.0),
//            Vec3::new(0.0, -1.0, 0.0),
//            Material::Lambertian(white),
//        )),
//        // floor
//        Box::new(Plane::new(
//            Vec3::new(0.0, 0.0, 0.0),
//            Vec3::new(0.0, 1.0, 0.0),
//            Material::Lambertian(white),
//        )),
//        // back wall
//        Box::new(Plane::new(
//            Vec3::new(0.0, 0.0, -10.0),
//            Vec3::new(0.0, 0.0, 1.0),
//            Material::Lambertian(white),
//        )),
//        // back wall (behind the camera)
//        Box::new(Plane::new(
//            Vec3::new(0.0, 0.0, 10.0),
//            Vec3::new(0.0, 0.0, -1.0),
//            Material::Lambertian(white),
//        )),
//        Box::new(Sphere::new(
//            Vec3::new(-2.5, 2.0, -3.0),
//            2.0,
//            Material::Dielectric(Color::new(1.0, 1.0, 1.0), 2.42),
//        )),
//        Box::new(Sphere::new(
//            Vec3::new(2.5, 2.0, -7.0),
//            2.0,
//            Material::Metal(gold, 0.25),
//        )),
//        Box::new(Sphere::new(
//            Vec3::new(3.8, 1.0, -2.5),
//            1.0,
//            Material::Lambertian(blue),
//        )),
//    ];
//
//    let look_from = Vec3::new(0.0, 5.0, 10.0);
//    let look_at = Vec3::new(0.0, 5.0, 0.0);
//
//    Scene {
//        camera: Camera::new(look_from, look_at, 50.0, aspect_ratio, 0.0),
//        objects: List::from_vec(objects),
//    }
//}

fn test_scene(aspect_ratio: f64) -> Scene {
    let objects: Vec<Hitable> = vec![
        Box::new(Sphere {
            center: Vec3::new(0.0, -5000.0, 0.0),
            radius: 5000.0,
            material: Material::Lambertian(Color::from_u8(0x66, 0x33, 0x99)),
        }),
        Box::new(Disc {
            center: Vec3::new(0.0, 50.0, 0.0),
            radius: 10.0,
            normal: Vec3::new(0.0, -1.0, 0.0),
            material: Material::DiffuseLight(Color::new(1.0, 1.0, 1.0) * 20.0),
        }),
        Box::new(Sphere {
            center: Vec3::new(0.0, 5.0, 0.0),
            radius: 3.0,
            material: Material::Lambertian(Color::new(0.05, 0.95, 0.05)),
        }),
    ];

    let look_from = Vec3::new(0.0, 10.0, 20.0);
    let look_at = Vec3::new(0.0, 5.0, 0.0);

    Scene {
        camera: Camera::new(look_from, look_at, 50.0, aspect_ratio, 0.0),
        objects: List::from_vec(objects),
    }
}

fn main() {
    let width = 640;
    let height = 480;
    let samples = 10;
    let aspect_ratio = f64::from(width) / f64::from(height);
    let gamma = 2.2f64;
    let max_depth = 10;
    let workers = 12;

    //let scene = cornell_box(aspect_ratio);
    let scene = raytracing_one_weekend(aspect_ratio);
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
