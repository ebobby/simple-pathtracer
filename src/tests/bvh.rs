//! The BVH must return exactly the nearest hit that brute force finds.

use crate::intersectable::Intersectable;
use crate::ray::Ray;
use crate::shape::{Disc, Sphere};
use crate::{Color, Hitable, Material, Texture, Vec3, BVH};

use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;

fn grey() -> Material {
    Material::lambertian(Texture::constant_color(Color::new(0.5, 0.5, 0.5)))
}

fn random_shapes(rng: &mut Xoshiro256PlusPlus, count: usize) -> Vec<Hitable> {
    (0..count)
        .map(|i| -> Hitable {
            let center = Vec3::new(
                rng.gen_range(-10.0..10.0),
                rng.gen_range(-10.0..10.0),
                rng.gen_range(-10.0..10.0),
            );
            let radius = rng.gen_range(0.1..2.0);
            if i % 4 == 0 {
                Box::new(Disc {
                    center,
                    normal: Vec3::new(
                        rng.gen_range(-1.0..1.0),
                        rng.gen_range(-1.0..1.0),
                        rng.gen_range(-1.0..1.0),
                    )
                    .normalize(),
                    radius,
                    material: grey(),
                })
            } else {
                Box::new(Sphere { center, radius, material: grey() })
            }
        })
        .collect()
}

fn random_ray(rng: &mut Xoshiro256PlusPlus) -> Ray {
    Ray {
        origin: Vec3::new(
            rng.gen_range(-15.0..15.0),
            rng.gen_range(-15.0..15.0),
            rng.gen_range(-15.0..15.0),
        ),
        direction: Vec3::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
        ),
    }
}

#[test]
fn bvh_matches_brute_force_nearest_hit() {
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(42);
    let brute = random_shapes(&mut rng, 300);
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(42);
    let bvh = BVH::from_vec(random_shapes(&mut rng, 300));

    let mut hits = 0;
    for _ in 0..5000 {
        let ray = random_ray(&mut rng);
        let expected = brute
            .iter()
            .filter_map(|s| s.intersect(&ray, 0.0001, f64::INFINITY))
            .min_by(|a, b| a.t.partial_cmp(&b.t).unwrap());
        let actual = bvh.intersect(&ray, 0.0001, f64::INFINITY);

        match (expected, actual) {
            (None, None) => {}
            (Some(e), Some(a)) => {
                hits += 1;
                assert!((e.t - a.t).abs() < 1e-9, "expected t {} got {}", e.t, a.t);
            }
            (e, a) => panic!("brute force {:?} vs bvh {:?}", e.map(|h| h.t), a.map(|h| h.t)),
        }
    }
    assert!(hits > 500, "only {hits} hits; scene too sparse to be meaningful");
}

#[test]
fn bvh_handles_single_object() {
    let bvh = BVH::from_vec(vec![Box::new(Sphere {
        center: Vec3::new(0.0, 0.0, -5.0),
        radius: 1.0,
        material: grey(),
    }) as Hitable]);
    let ray = Ray { origin: Vec3::zero(), direction: Vec3::new(0.0, 0.0, -1.0) };
    let hit = bvh.intersect(&ray, 0.0001, f64::INFINITY).expect("should hit");
    assert!((hit.t - 4.0).abs() < 1e-9);
}
