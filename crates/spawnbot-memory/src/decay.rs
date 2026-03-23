/// Compute decay factor: e^(-lambda * age_days) where lambda = ln(2) / half_life_days
pub fn decay_factor(age_days: f64, half_life_days: u32) -> f64 {
    let lambda = (2.0_f64).ln() / half_life_days as f64;
    (-lambda * age_days).exp()
}

/// Apply temporal decay to a score. Evergreen memories are exempt.
pub fn apply_decay(score: f64, age_days: f64, half_life_days: u32, evergreen: bool) -> f64 {
    if evergreen {
        score
    } else {
        score * decay_factor(age_days, half_life_days)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-6;

    #[test]
    fn test_decay_at_zero_age() {
        let factor = decay_factor(0.0, 30);
        assert!((factor - 1.0).abs() < EPSILON, "Decay at age 0 should be 1.0, got {factor}");
    }

    #[test]
    fn test_decay_at_half_life() {
        let half_life = 30;
        let factor = decay_factor(half_life as f64, half_life);
        assert!(
            (factor - 0.5).abs() < EPSILON,
            "Decay at half_life should be 0.5, got {factor}"
        );
    }

    #[test]
    fn test_decay_at_double_half_life() {
        let half_life = 30;
        let factor = decay_factor((half_life * 2) as f64, half_life);
        assert!(
            (factor - 0.25).abs() < EPSILON,
            "Decay at 2x half_life should be 0.25, got {factor}"
        );
    }

    #[test]
    fn test_evergreen_bypass() {
        let score = 10.0;
        let result = apply_decay(score, 1000.0, 30, true);
        assert!(
            (result - score).abs() < EPSILON,
            "Evergreen memories should not decay, got {result}"
        );
    }

    #[test]
    fn test_apply_decay_non_evergreen() {
        let score = 10.0;
        let half_life = 30;
        let result = apply_decay(score, half_life as f64, half_life, false);
        assert!(
            (result - 5.0).abs() < EPSILON,
            "Score should halve at half_life, got {result}"
        );
    }
}
