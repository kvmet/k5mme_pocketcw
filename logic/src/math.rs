//! Math that `core` lacks for `f32`.

/// Square root by Newton's method.
pub fn sqrt(x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    let mut g = x.max(1.0);
    for _ in 0..20 {
        g = 0.5 * (g + x / g);
    }
    g
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqrt_matches_std_across_the_range_used() {
        for x in [1e-6f32, 0.25, 2.0, 9.0, 1e4, 1e6] {
            assert!((sqrt(x) - x.sqrt()).abs() <= x.sqrt() * 1e-6, "{x}");
        }
        assert_eq!(sqrt(-1.0), 0.0);
    }
}
