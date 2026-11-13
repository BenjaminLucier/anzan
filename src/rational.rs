//! Exact fractions, so that answers like 1/8, 0.125 and 12.5% can be
//! compared without floating-point rounding.

use std::{cmp::Ordering, fmt, str::FromStr};

/// The most decimal places `to_decimal` supports. Ample for mental math,
/// and small enough that scaling cannot overflow for sensible values.
pub const MAX_PLACES: u32 = 18;

/// A fraction in lowest terms, with a positive denominator.
///
/// Arithmetic is checked and returns `None` on overflow. With `i128` that
/// is far beyond anything a mental-math question needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rational {
    num: i128,
    den: i128,
}

impl Rational {
    pub const ZERO: Self = Self { num: 0, den: 1 };
    pub const ONE: Self = Self { num: 1, den: 1 };

    /// `num / den` in lowest terms, or `None` if `den` is zero.
    pub fn new(num: i128, den: i128) -> Option<Self> {
        if den == 0 {
            return None;
        }
        let g = gcd(num, den).max(1);
        let sign = den.signum();
        Some(Self {
            num: sign * num / g,
            den: sign * den / g,
        })
    }

    pub const fn int(n: i64) -> Self {
        Self {
            num: n as i128,
            den: 1,
        }
    }

    pub fn num(self) -> i128 {
        self.num
    }

    pub fn den(self) -> i128 {
        self.den
    }

    pub fn is_integer(self) -> bool {
        self.den == 1
    }

    pub fn to_f64(self) -> f64 {
        self.num as f64 / self.den as f64
    }

    pub fn abs(self) -> Self {
        Self {
            num: self.num.abs(),
            den: self.den,
        }
    }

    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        let num = self
            .num
            .checked_mul(rhs.den)?
            .checked_add(rhs.num.checked_mul(self.den)?)?;
        Self::new(num, self.den.checked_mul(rhs.den)?)
    }

    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.checked_add(Self {
            num: -rhs.num,
            den: rhs.den,
        })
    }

    pub fn checked_mul(self, rhs: Self) -> Option<Self> {
        Self::new(
            self.num.checked_mul(rhs.num)?,
            self.den.checked_mul(rhs.den)?,
        )
    }

    pub fn checked_div(self, rhs: Self) -> Option<Self> {
        Self::new(
            self.num.checked_mul(rhs.den)?,
            self.den.checked_mul(rhs.num)?,
        )
    }

    /// Whether the decimal expansion ends, as for 1/8 = 0.125 but not for
    /// 1/3 = 0.333…: true when the denominator has no prime factors but 2
    /// and 5.
    pub fn terminates(self) -> bool {
        self.exact_places().is_some()
    }

    /// How many decimal places the expansion needs if it ends: the larger
    /// of the powers of 2 and 5 in the denominator.
    fn exact_places(self) -> Option<u32> {
        let mut den = self.den;
        let mut count = [0; 2];
        for (p, count) in [2, 5].into_iter().zip(&mut count) {
            while den % p == 0 {
                den /= p;
                *count += 1;
            }
        }
        (den == 1).then(|| count[0].max(count[1]))
    }

    /// The decimal expansion to at most `places` places, rounded half away
    /// from zero, without trailing zeros.
    ///
    /// # Panics
    ///
    /// Panics if `places` exceeds `MAX_PLACES`.
    pub fn to_decimal(self, places: u32) -> String {
        assert!(places <= MAX_PLACES, "too many decimal places");
        let scale = 10i128.pow(places);
        let scaled = (2 * self.num.abs() * scale + self.den) / (2 * self.den);
        let (int, mut frac) = (scaled / scale, scaled % scale);
        let mut digits = places;
        while digits > 0 && frac % 10 == 0 {
            frac /= 10;
            digits -= 1;
        }
        let sign = if self.num < 0 && scaled != 0 { "-" } else { "" };
        if digits == 0 {
            format!("{sign}{int}")
        } else {
            format!("{sign}{int}.{frac:0width$}", width = digits as usize)
        }
    }
}

impl From<i64> for Rational {
    fn from(n: i64) -> Self {
        Self::int(n)
    }
}

impl Ord for Rational {
    fn cmp(&self, other: &Self) -> Ordering {
        // Denominators are positive, so cross-multiplying keeps the order.
        (self.num * other.den).cmp(&(other.num * self.den))
    }
}

impl PartialOrd for Rational {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Integers as integers, terminating decimals as decimals, and anything
/// else as a fraction: `3`, `0.125`, `1/7`.
impl fmt::Display for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_integer() {
            write!(f, "{}", self.num)
        } else if let Some(places) = self.exact_places().filter(|&p| p <= MAX_PLACES) {
            f.write_str(&self.to_decimal(places))
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseRationalError;

impl fmt::Display for ParseRationalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("not a number")
    }
}

/// Parses integers (`-12`, `1,000`), decimals (`0.125`, `.5`) and
/// fractions (`3/8`, `-1/7`).
impl FromStr for Rational {
    type Err = ParseRationalError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s: String = s.trim().chars().filter(|&c| c != ',').collect();
        match s.split_once('/') {
            Some((num, den)) => {
                let num = parse_decimal(num).ok_or(ParseRationalError)?;
                let den = parse_decimal(den).ok_or(ParseRationalError)?;
                num.checked_div(den).ok_or(ParseRationalError)
            }
            None => parse_decimal(&s).ok_or(ParseRationalError),
        }
    }
}

/// `-12`, `0.125`, `.5`, `3.`: an optional minus sign, then digits with at
/// most one decimal point.
fn parse_decimal(s: &str) -> Option<Rational> {
    // More digits than this could overflow i128.
    const MAX_DIGITS: usize = 30;

    let (negative, s) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s),
    };
    let (int, frac) = s.split_once('.').unwrap_or((s, ""));
    let digits = int.len() + frac.len();
    if digits == 0
        || digits > MAX_DIGITS
        || !int.bytes().chain(frac.bytes()).all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let mut num: i128 = 0;
    for b in int.bytes().chain(frac.bytes()) {
        num = num * 10 + i128::from(b - b'0');
    }
    let num = if negative { -num } else { num };
    Rational::new(num, 10i128.pow(frac.len() as u32))
}

fn gcd(mut a: i128, mut b: i128) -> i128 {
    (a, b) = (a.abs(), b.abs());
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn r(num: i128, den: i128) -> Rational {
        Rational::new(num, den).unwrap()
    }

    fn parse(s: &str) -> Option<Rational> {
        s.parse().ok()
    }

    #[test]
    fn lowest_terms_with_positive_denominator() {
        assert_eq!(r(6, -8), r(-3, 4));
        assert_eq!((r(6, -8).num(), r(6, -8).den()), (-3, 4));
        assert_eq!(r(0, -5), Rational::ZERO);
        assert_eq!(Rational::new(1, 0), None);
    }

    #[test]
    fn parses_every_way_of_writing_a_number() {
        assert_eq!(parse("42"), Some(r(42, 1)));
        assert_eq!(parse("-42"), Some(r(-42, 1)));
        assert_eq!(parse("1,000"), Some(r(1000, 1)));
        assert_eq!(parse("0.125"), Some(r(1, 8)));
        assert_eq!(parse(".5"), Some(r(1, 2)));
        assert_eq!(parse("3."), Some(r(3, 1)));
        assert_eq!(parse("-0.5"), Some(r(-1, 2)));
        assert_eq!(parse("3/8"), Some(r(3, 8)));
        assert_eq!(parse("-1/7"), Some(r(-1, 7)));
        assert_eq!(parse(" 7 "), Some(r(7, 1)));
    }

    #[test]
    fn rejects_what_is_not_a_number() {
        for s in [
            "", "-", ".", "1.2.3", "1/0", "a", "1/", "/2", "--1", "1e3", "½",
        ] {
            assert_eq!(parse(s), None, "{s:?}");
        }
        assert_eq!(parse(&"9".repeat(31)), None);
    }

    #[test]
    fn display_picks_the_natural_form() {
        assert_eq!(r(3, 1).to_string(), "3");
        assert_eq!(r(1, 8).to_string(), "0.125");
        assert_eq!(r(-1, 16).to_string(), "-0.0625");
        assert_eq!(r(1, 7).to_string(), "1/7");
        assert_eq!(r(-5, 6).to_string(), "-5/6");
    }

    #[test]
    fn decimal_rounding() {
        assert_eq!(r(1, 7).to_decimal(6), "0.142857");
        assert_eq!(r(2, 3).to_decimal(4), "0.6667");
        assert_eq!(r(-2, 3).to_decimal(2), "-0.67");
        assert_eq!(r(1, 2).to_decimal(4), "0.5");
        assert_eq!(r(-1, 1000).to_decimal(2), "0");
        assert_eq!(r(995, 1000).to_decimal(2), "1");
    }

    #[test]
    fn terminating_decimals() {
        assert!(r(1, 8).terminates());
        assert!(r(7, 20).terminates());
        assert!(!r(1, 3).terminates());
        assert!(!r(1, 14).terminates());
    }

    #[test]
    fn arithmetic() {
        assert_eq!(r(1, 2).checked_add(r(1, 3)), Some(r(5, 6)));
        assert_eq!(r(1, 2).checked_sub(r(1, 3)), Some(r(1, 6)));
        assert_eq!(r(2, 3).checked_mul(r(3, 4)), Some(r(1, 2)));
        assert_eq!(r(2, 3).checked_div(r(4, 3)), Some(r(1, 2)));
        assert_eq!(r(2, 3).checked_div(Rational::ZERO), None);
        assert!(r(1, 3) < r(1, 2));
        assert!(r(-1, 2) < r(-1, 3));
    }

    fn rational() -> impl Strategy<Value = Rational> {
        (-1_000_000i128..1_000_000, 1i128..10_000).prop_map(|(n, d)| r(n, d))
    }

    proptest! {
        #[test]
        fn display_parses_back(x in rational()) {
            prop_assert_eq!(parse(&x.to_string()), Some(x));
        }

        #[test]
        fn add_then_sub_round_trips(a in rational(), b in rational()) {
            prop_assert_eq!(a.checked_add(b).unwrap().checked_sub(b), Some(a));
        }

        #[test]
        fn decimal_is_within_half_a_unit(x in rational()) {
            let d: f64 = x.to_decimal(4).parse().unwrap();
            prop_assert!((d - x.to_f64()).abs() <= 0.5e-4 + 1e-12);
        }
    }
}
