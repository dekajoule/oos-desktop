// SPDX-License-Identifier: GPL-2.0-or-later
//! Numbers as a person reads them on a scope: engineering prefixes, and the 1-2-5 steps of a scale knob.

/// `0.005` with unit `s/div` -> `5 ms/div`. A factor (`×`) gets no prefix.
pub fn eng(x: f64, unit: &str) -> String {
    if unit == "×" {
        return format!("{}×", trim(format!("{x:.3}")));
    }
    let a = x.abs();
    let (k, p) = [(1e-12, "p"), (1e-9, "n"), (1e-6, "µ"), (1e-3, "m"), (1.0, ""), (1e3, "k"), (1e6, "M"), (1e9, "G")]
        .into_iter()
        .rev()
        .find(|(k, _)| a >= *k * 0.9995)
        .unwrap_or((1.0, ""));
    let t = trim(format!("{:.3}", x / k));
    format!("{t} {p}{unit}")
}

fn trim(t: String) -> String {
    if t.contains('.') {
        t.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        t
    }
}

/// The next value of a 1-2-5 sequence above (`up`) or below `x`; the scope takes the nearest scale it has.
pub fn step125(x: f64, up: bool) -> f64 {
    if !(x.is_finite() && x > 0.0) {
        return x;
    }
    let decade = 10f64.powf(x.log10().floor());
    let steps = [1.0, 2.0, 5.0, 10.0, 20.0];
    // the step x is on, allowing for the rounding of the answer
    let m = x / decade;
    if up {
        let next = steps.iter().find(|&&s| s > m * 1.001).copied().unwrap_or(10.0);
        next * decade
    } else {
        let below = [0.5, 1.0, 2.0, 5.0, 10.0];
        let prev = below.iter().rev().find(|&&s| s < m * 0.999).copied().unwrap_or(0.5);
        prev * decade
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engineering_units() {
        assert_eq!(eng(0.005, "s/div"), "5 ms/div");
        assert_eq!(eng(2.0, "V"), "2 V");
        assert_eq!(eng(-1.5e-7, "s"), "-150 ns");
        assert_eq!(eng(0.0, "V"), "0 V");
        assert_eq!(eng(1.25e9, "Sa/s"), "1.25 GSa/s");
        assert_eq!(eng(10.0, "×"), "10×");
        assert_eq!(eng(0.9999999, "V"), "1 V");
    }

    #[test]
    fn scale_steps_go_one_two_five() {
        let close = |a: f64, b: f64| (a - b).abs() < b * 1e-9;
        assert!(close(step125(1e-3, true), 2e-3));
        assert!(close(step125(2e-3, true), 5e-3));
        assert!(close(step125(5e-3, true), 1e-2));
        assert!(close(step125(1e-2, false), 5e-3));
        assert!(close(step125(0.5, false), 0.2));
        assert!(close(step125(0.2, false), 0.1));
        // a value between steps goes to the next step in that direction
        assert!(close(step125(0.3, true), 0.5));
        assert!(close(step125(0.3, false), 0.2));
        // the rounding of an answer does not skip a step
        assert!(close(step125(0.499_999_9, true), 1.0));
        assert!(close(step125(0.500_000_1, false), 0.2));
        assert_eq!(step125(0.0, true), 0.0);
    }
}
