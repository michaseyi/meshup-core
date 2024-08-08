use roots::{find_roots_quartic, Roots};

#[derive(Debug, PartialEq)]
pub enum QuarticRoots {
    None,
    One([f32; 1]),
    Two([f32; 2]),
    Three([f32; 3]),
    Four([f32; 4]),
}

impl QuarticRoots {
    pub fn max(&self) -> Option<f32> {
        match self {
            QuarticRoots::None => None,
            QuarticRoots::One([a]) => Some(*a),
            QuarticRoots::Two([a, b]) => Some(a.max(*b)),
            QuarticRoots::Three([a, b, c]) => Some(a.max(*b).max(*c)),
            QuarticRoots::Four([a, b, c, d]) => Some(a.max(*b).max(*c).max(*d)),
        }
    }

    pub fn min(&self) -> Option<f32> {
        match self {
            QuarticRoots::None => None,
            QuarticRoots::One([a]) => Some(*a),
            QuarticRoots::Two([a, b]) => Some(a.min(*b)),
            QuarticRoots::Three([a, b, c]) => Some(a.min(*b).min(*c)),
            QuarticRoots::Four([a, b, c, d]) => Some(a.min(*b).min(*c).min(*d)),
        }
    }
}

///Reference https://en.wikipedia.org/wiki/Quartic_equation
// pub fn solve_quartic(a4: f32, a3: f32, a2: f32, a1: f32, a0: f32) -> QuarticRoots {
//     // a4*x^4 + a3*x^3 + a2*x^2 + a1*x + a0 = 0
//     // x = u - a3 / 4a4

//     // Depressed quartic coefficients
//     let a = ((-3. * a3.powi(2)) / (8. * a4.powi(2))) + (a2 / a4);
//     let b = (a3.powi(3) / (8. * a4.powi(3))) - ((a3 * a2) / (2. * a4.powi(2))) + (a1 / a4);
//     let c = ((-3. * a3.powi(4)) / (256. * a4.powi(4))) + ((a2 * a3.powi(2)) / (16. * a4.powi(3)))
//         - ((a3 * a1) / (4. * a4.powi(2)))
//         + (a0 / a4);

//     // Depressed quartic equation becomes u^4 + a*u^2 + b*u + c = 0

//     let p = -(a.powi(2) / 12.) - c;
//     let q = -(a.powi(3) / 108.) + ((a * c) / 3.) - (b.powi(2) / 8.);

//     let w = (-(q / 2.) + ((q.powi(2) / 4.) + (p.powi(3) / 27.)).sqrt()).cbrt();

//     let y = (a / 6.) + w - (p / (3. * w));

//     let two_y_minus_a_sqrt = (2. * y - a).sqrt();

//     if !two_y_minus_a_sqrt.is_finite() {
//         return QuarticRoots::None;
//     }

//     let two_y_minus_a_sqrt_recip = two_y_minus_a_sqrt.recip();

//     let u0 = (-two_y_minus_a_sqrt + (-(2. * y) - a + ((2. * b) * two_y_minus_a_sqrt_recip)).sqrt())
//         * 0.5;
//     let u1 = (-two_y_minus_a_sqrt - (-(2. * y) - a + ((2. * b) * two_y_minus_a_sqrt_recip)).sqrt())
//         * 0.5;
//     let u2 =
//         (two_y_minus_a_sqrt + (-(2. * y) - a - ((2. * b) * two_y_minus_a_sqrt_recip)).sqrt()) * 0.5;
//     let u3 =
//         (two_y_minus_a_sqrt - (-(2. * y) - a - ((2. * b) * two_y_minus_a_sqrt_recip)).sqrt()) * 0.5;

//     let d = a3 / (4. * a4);

//     let x0 = u0 - d;
//     let x1 = u1 - d;
//     let x2 = u2 - d;
//     let x3 = u3 - d;

//     let roots = [x0, x1, x2, x3];
//     let mut valid_roots = [0.; 4];
//     let mut valid_count = 0;

//     for i in 0..4 {
//         if roots[i].is_finite() {
//             valid_roots[valid_count] = roots[i];
//             valid_count += 1;
//         }
//     }

//     match valid_count {
//         0 => QuarticRoots::None,
//         1 => QuarticRoots::One(valid_roots[0]),
//         2 => QuarticRoots::Two(valid_roots[0], valid_roots[1]),
//         3 => QuarticRoots::Three(valid_roots[0], valid_roots[1], valid_roots[2]),
//         4 => QuarticRoots::Four(
//             valid_roots[0],
//             valid_roots[1],
//             valid_roots[2],
//             valid_roots[3],
//         ),
//         _ => QuarticRoots::None,
//     }
// }

pub fn solve_quartic(a: f32, b: f32, c: f32, d: f32, e: f32) -> QuarticRoots {
    match find_roots_quartic(a as f64, b as f64, c as f64, d as f64, e as f64) {
        Roots::Four(roots) => QuarticRoots::Four(roots.map(|f| f as f32)),
        Roots::Three(roots) => QuarticRoots::Three(roots.map(|f| f as f32)),
        Roots::Two(roots) => QuarticRoots::Two(roots.map(|f| f as f32)),
        Roots::One(roots) => QuarticRoots::One(roots.map(|f| f as f32)),
        Roots::No([]) => QuarticRoots::None,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_integer_quartic_roots() {
        let actual_roots = solve_quartic(1., -5., -15., 5., 14.);
        let expected_roots = [-2.0, -1.0, 1.0, 7.0];

        match actual_roots {
            QuarticRoots::Four(roots) => {
                for (actual, expected) in roots.iter().zip(expected_roots.iter()) {
                    assert!((actual - expected).abs() < 0.0001);
                }
            }
            _ => panic!(),
        };
    }

    #[test]
    fn test_decimal_quartic_roots() {
        let actual_roots = solve_quartic(1., -5., -15., 5., 1.);
        let expected_roots = [-2.318769, -0.142252, 0.431266, 7.029735f32];

        match actual_roots {
            QuarticRoots::Four(roots) => {
                for (actual, expected) in roots.iter().zip(expected_roots.iter()) {
                    assert!((actual - expected).abs() < 0.0001);
                }
            }
            _ => panic!(),
        };
    }
}
