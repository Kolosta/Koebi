//! Numerical tools: real roots of quadratics, adaptive Gauss–Legendre integration.
//! Nothing allocates: roots come back in a fixed array.

/// Up to two real roots, in ascending order.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Roots {
    values: [f64; 2],
    len: usize,
}

impl Roots {
    fn push(&mut self, value: f64) {
        if self.len < self.values.len() && value.is_finite() {
            self.values[self.len] = value;
            self.len += 1;
        }
    }

    fn sorted(mut self) -> Self {
        self.values[..self.len].sort_unstable_by(f64::total_cmp);
        self
    }

    /// The roots.
    pub(crate) fn as_slice(&self) -> &[f64] {
        &self.values[..self.len]
    }
}

/// The real roots of `a·t² + b·t + c`, falling back to the linear equation when `a` is 0.
pub(crate) fn quadratic(a: f64, b: f64, c: f64) -> Roots {
    let mut roots = Roots::default();
    let scale = a.abs().max(b.abs()).max(c.abs());
    if scale == 0.0 {
        return roots;
    }
    if a.abs() <= scale * 1e-12 {
        if b.abs() > scale * 1e-12 {
            roots.push(-c / b);
        }
        return roots;
    }
    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 {
        return roots;
    }
    if disc == 0.0 {
        roots.push(-b / (2.0 * a));
        return roots;
    }
    // The stable form: no cancellation between -b and the square root.
    let q = -0.5 * (b + b.signum() * disc.sqrt());
    roots.push(q / a);
    if q != 0.0 {
        roots.push(c / q);
    }
    roots.sorted()
}

/// Nodes and weights of the 8-point Gauss–Legendre rule on `[-1, 1]`.
const GAUSS_8: [(f64, f64); 8] = [
    (-0.960_289_856_497_536_3, 0.101_228_536_290_376_26),
    (-0.796_666_477_413_626_7, 0.222_381_034_453_374_48),
    (-0.525_532_409_916_329, 0.313_706_645_877_887_27),
    (-0.183_434_642_495_649_8, 0.362_683_783_378_362),
    (0.183_434_642_495_649_8, 0.362_683_783_378_362),
    (0.525_532_409_916_329, 0.313_706_645_877_887_27),
    (0.796_666_477_413_626_7, 0.222_381_034_453_374_48),
    (0.960_289_856_497_536_3, 0.101_228_536_290_376_26),
];

fn gauss(f: &impl Fn(f64) -> f64, a: f64, b: f64) -> f64 {
    let (mid, half) = (a.midpoint(b), (b - a) * 0.5);
    GAUSS_8
        .iter()
        .map(|&(x, w)| w * f(mid + half * x))
        .sum::<f64>()
        * half
}

/// The integral of `f` over `[a, b]` within `accuracy` (absolute), by halving the interval
/// where two estimates disagree, down to a fixed depth.
pub(crate) fn integrate(f: &impl Fn(f64) -> f64, a: f64, b: f64, accuracy: f64) -> f64 {
    fn recurse(
        f: &impl Fn(f64) -> f64,
        a: f64,
        b: f64,
        whole: f64,
        accuracy: f64,
        depth: u32,
    ) -> f64 {
        let mid = a.midpoint(b);
        let (left, right) = (gauss(f, a, mid), gauss(f, mid, b));
        if depth == 0 || (left + right - whole).abs() <= accuracy {
            return left + right;
        }
        recurse(f, a, mid, left, accuracy * 0.5, depth - 1)
            + recurse(f, mid, b, right, accuracy * 0.5, depth - 1)
    }
    recurse(f, a, b, gauss(f, a, b), accuracy.max(1e-15), 24)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quadratics_find_their_real_roots() {
        assert_eq!(quadratic(1.0, -3.0, 2.0).as_slice(), &[1.0, 2.0]);
        assert_eq!(quadratic(0.0, 2.0, -4.0).as_slice(), &[2.0]);
        assert_eq!(quadratic(1.0, 0.0, 1.0).as_slice(), &[] as &[f64]);
    }

    #[test]
    fn integration_reaches_its_accuracy() {
        let area = integrate(&|t: f64| t.sin(), 0.0, core::f64::consts::PI, 1e-12);
        assert!((area - 2.0).abs() < 1e-12, "{area}");
        let peaked = integrate(&|t: f64| 1.0 / (1e-4 + t * t), -1.0, 1.0, 1e-9);
        let exact = 2.0 * (1.0 / 1e-2) * (1.0_f64 / 1e-2).atan();
        assert!((peaked - exact).abs() < 1e-7 * exact, "{peaked} {exact}");
    }
}
