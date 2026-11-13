//! Mathematical notation for questions: just enough structure to draw
//! powers, fractions and roots properly instead of as `^`, `/` and `sqrt`.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    /// Numbers and operators, such as `47 × 8`.
    Text(String),
    /// Expressions side by side.
    Row(Vec<Expr>),
    /// A base raised to an exponent.
    Pow(Box<Expr>, Box<Expr>),
    /// A numerator over a denominator.
    Frac(Box<Expr>, Box<Expr>),
    Sqrt(Box<Expr>),
}

impl Expr {
    pub fn text(s: impl Into<String>) -> Self {
        Expr::Text(s.into())
    }

    pub fn pow(base: Expr, exp: Expr) -> Self {
        Expr::Pow(Box::new(base), Box::new(exp))
    }

    pub fn frac(num: Expr, den: Expr) -> Self {
        Expr::Frac(Box::new(num), Box::new(den))
    }

    pub fn sqrt(inner: Expr) -> Self {
        Expr::Sqrt(Box::new(inner))
    }
}

/// On one line, as stored in history: `17²`, `3/8`, `√50`.
impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Text(s) => f.write_str(s),
            Expr::Row(parts) => parts.iter().try_for_each(|p| write!(f, "{p}")),
            Expr::Pow(base, exp) => {
                let exp = exp.to_string();
                match superscript(&exp) {
                    Some(sup) => write!(f, "{base}{sup}"),
                    None => write!(f, "{base}^{exp}"),
                }
            }
            Expr::Frac(num, den) => write!(f, "{num}/{den}"),
            Expr::Sqrt(inner) => write!(f, "√{inner}"),
        }
    }
}

/// `s` in superscript digits, if it is all digits.
pub fn superscript(s: &str) -> Option<String> {
    const DIGITS: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
    s.bytes()
        .map(|b| b.is_ascii_digit().then(|| DIGITS[usize::from(b - b'0')]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displays_on_one_line() {
        let t = Expr::text;
        assert_eq!(Expr::pow(t("17"), t("2")).to_string(), "17²");
        assert_eq!(Expr::pow(t("2"), t("n")).to_string(), "2^n");
        assert_eq!(Expr::frac(t("3"), t("8")).to_string(), "3/8");
        assert_eq!(Expr::sqrt(t("50")).to_string(), "√50");
        let row = Expr::Row(vec![t("1000 × "), Expr::pow(t("1.1"), t("3"))]);
        assert_eq!(row.to_string(), "1000 × 1.1³");
    }

    #[test]
    fn superscripts() {
        assert_eq!(superscript("20").as_deref(), Some("²⁰"));
        assert_eq!(superscript("x"), None);
    }
}
