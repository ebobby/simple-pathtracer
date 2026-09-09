//! Exposure and tone curves must behave as documented before gamma encoding.

use crate::{Color, ToneCurve, Tonemap};

fn grey(v: f64) -> Color {
    Color::new(v, v, v)
}

#[test]
fn clamp_curve_is_identity_below_one_and_clips_above() {
    assert!((ToneCurve::Clamp.apply(0.5) - 0.5).abs() < 1e-12);
    assert_eq!(ToneCurve::Clamp.apply(3.0), 1.0);
    assert_eq!(ToneCurve::Clamp.apply(-1.0), 0.0);
}

#[test]
fn reinhard_maps_one_to_a_half_and_never_reaches_one() {
    assert!((ToneCurve::Reinhard.apply(1.0) - 0.5).abs() < 1e-12);
    assert!(ToneCurve::Reinhard.apply(1000.0) < 1.0);
    assert_eq!(ToneCurve::Reinhard.apply(0.0), 0.0);
}

#[test]
fn aces_is_monotonic_and_saturates_near_one() {
    assert_eq!(ToneCurve::Aces.apply(0.0), 0.0);
    let mut previous = 0.0;
    for i in 1..=200 {
        let x = i as f64 * 0.1;
        let y = ToneCurve::Aces.apply(x);
        assert!(y >= previous, "not monotonic at {x}");
        assert!(y <= 1.0);
        previous = y;
    }
    assert!(ToneCurve::Aces.apply(20.0) > 0.99);
    // Mid-grey input stays mid-grey-ish rather than being crushed
    let mid = ToneCurve::Aces.apply(0.18);
    assert!((0.15..0.35).contains(&mid), "ACES(0.18) = {mid}");
}

#[test]
fn exposure_scales_before_the_curve_and_gamma_encodes_after() {
    let tonemap = Tonemap {
        exposure: 2.0,
        curve: ToneCurve::Clamp,
        gamma: 2.2,
        bloom: None,
    };
    // 0.25 * 2 = 0.5, then 0.5^(1/2.2) * 255 ≈ 186
    let rgb = tonemap.apply(grey(0.25));
    assert_eq!(rgb.0, [186, 186, 186]);

    let bright = tonemap.apply(grey(4.0));
    assert_eq!(bright.0, [255, 255, 255]);
}

#[test]
fn default_tonemap_matches_the_old_output_stage() {
    let old = grey(0.3).to_gamma_rgb(1.0 / 2.2);
    let new = Tonemap::default().apply(grey(0.3));
    assert_eq!(old.0, new.0);
}
