use pathtracer::shape::*;
use pathtracer::Camera;
use pathtracer::Color;
use pathtracer::Hitable;
use pathtracer::Material;
use pathtracer::Scene;
use pathtracer::Texture;
use pathtracer::Vec3;
use pathtracer::BVH;

fn raytracing_one_weekend(aspect_ratio: f64) -> Scene {
    let mut list: Vec<Hitable> = Vec::new();
    let radius = 0.2f64;

    list.push(Box::new(Sphere {
        center: Vec3::new(0.0, -1000.0, 0.0),
        radius: 1000.0,
        material: Material::lambertian(Texture::constant_color(Color::new(0.5, 0.5, 0.5))),
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
                        material: Material::lambertian(Texture::constant_color(Color::new(
                            rand::random::<f64>() * rand::random::<f64>(),
                            rand::random::<f64>() * rand::random::<f64>(),
                            rand::random::<f64>() * rand::random::<f64>(),
                        ))),
                    }));
                } else if choose_mat < 0.95 {
                    list.push(Box::new(Sphere {
                        center,
                        radius,
                        material: Material::metal(
                            Texture::constant_color(Color::new(
                                0.5 * (1.0 + rand::random::<f64>()),
                                0.5 * (1.0 + rand::random::<f64>()),
                                0.5 * (1.0 + rand::random::<f64>()),
                            )),
                            0.5 * rand::random::<f64>(),
                        ),
                    }));
                } else {
                    list.push(Box::new(Sphere {
                        center,
                        radius,
                        material: Material::dielectric(
                            Texture::constant_color(Color::new(1.0, 1.0, 1.0)),
                            1.5,
                        ),
                    }));
                }
            }
        }
    }

    list.push(Box::new(Sphere {
        center: Vec3::new(0.0, 1.0, 0.0),
        radius: 1.0,
        material: Material::dielectric(Texture::constant_color(Color::new(1.0, 1.0, 1.0)), 1.5),
    }));
    list.push(Box::new(Sphere {
        center: Vec3::new(-4.0, 1.0, 0.0),
        radius: 1.0,
        material: Material::lambertian(Texture::constant_color(Color::new(0.4, 0.2, 0.1))),
    }));
    list.push(Box::new(Sphere {
        center: Vec3::new(4.0, 1.0, 0.0),
        radius: 1.0,
        material: Material::metal(Texture::constant_color(Color::new(0.7, 0.6, 0.5)), 0.0),
    }));

    list.push(Box::new(Sphere {
        center: Vec3::new(0.0, 0.0, 0.0),
        radius: 5000.0,
        material: Material::diffuse_light(Texture::constant_color(Color::new(0.5, 0.7, 1.0))),
    }));

    let look_from = Vec3::new(13.0, 2.0, 3.0);
    let look_at = Vec3::new(0.0, 0.0, 0.0);

    Scene {
        camera: Camera::new(look_from, look_at, 20.0, aspect_ratio, 0.0),
        world: BVH::from_vec(list),
    }
}

fn main() {
    let width = 640;
    let height = 480;
    let samples = 1000;
    let aspect_ratio = f64::from(width) / f64::from(height);
    let gamma = 2.2f64;
    let max_depth = 10;
    let workers: usize = 12;

    let scene = raytracing_one_weekend(aspect_ratio);

    pathtracer::render(
        scene,
        width,
        height,
        samples,
        max_depth,
        gamma,
        workers,
        "output/one-weekend.png",
    );
}
