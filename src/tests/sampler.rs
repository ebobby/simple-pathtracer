//! The 2D Sobol sampler must keep its stratification after scrambling and
//! index shuffling: any 2^k consecutive points from the start put exactly
//! one point in every elementary interval of area 2^-k.

use crate::sampler::{sobol_2d, sobol_dim1_reference, Sampler};

/// Check the (0,2)-sequence property for `points` (in [0,1)²) of length 2^k.
fn assert_stratified(points: &[(f64, f64)]) {
    let n = points.len();
    let k = n.trailing_zeros();
    assert_eq!(1 << k, n, "need a power of two number of points");

    for a in 0..=k {
        let (cols, rows) = (1usize << a, 1usize << (k - a));
        let mut counts = vec![0u32; n];
        for &(x, y) in points {
            assert!((0.0..1.0).contains(&x) && (0.0..1.0).contains(&y), "({x}, {y})");
            let cx = (x * cols as f64) as usize;
            let cy = (y * rows as f64) as usize;
            counts[cy * cols + cx] += 1;
        }
        assert!(
            counts.iter().all(|&c| c == 1),
            "{cols}x{rows} elementary intervals not each hit once: {counts:?}"
        );
    }
}

#[test]
fn plain_sobol_2d_is_a_02_sequence() {
    for k in [2u32, 4, 6, 8] {
        let n = 1usize << k;
        let points: Vec<(f64, f64)> = (0..n as u32)
            .map(|i| {
                let (x, y) = sobol_2d(i);
                (f64::from(x) / 4294967296.0, f64::from(y) / 4294967296.0)
            })
            .collect();
        assert_stratified(&points);
    }
}

#[test]
fn scrambled_shuffled_sampler_keeps_stratification() {
    for pixel_seed in [0u32, 1, 12345, 0xdead_beef] {
        for slot in 0..4 {
            let n = 64;
            let points: Vec<(f64, f64)> = (0..n)
                .map(|index| Sampler::new(pixel_seed, index).get_2d(slot))
                .collect();
            assert_stratified(&points);
        }
    }
}

#[test]
fn different_slots_and_pixels_give_different_points() {
    let a = Sampler::new(7, 3).get_2d(0);
    let b = Sampler::new(7, 3).get_2d(1);
    let c = Sampler::new(8, 3).get_2d(0);
    assert_ne!(a, b);
    assert_ne!(a, c);
}

#[test]
fn butterfly_matches_direction_number_recurrence() {
    for index in (0..5000u32).chain([0xffff_ffff, 0x8000_0000, 0x1234_5678, 0xdead_beef]) {
        assert_eq!(sobol_2d(index).1, sobol_dim1_reference(index), "index {index}");
    }
}
