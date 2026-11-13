//! Small number-theory helpers.

/// Whether `n` is prime, by trial division. Fast for any `n` a person
/// could factor in their head.
pub fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    (2..)
        .take_while(|d| d * d <= n)
        .all(|d| !n.is_multiple_of(d))
}

/// The prime factors of `n`, repeated by multiplicity, ascending. Empty
/// for 0 and 1.
pub fn prime_factors(mut n: u64) -> Vec<u64> {
    let mut factors = Vec::new();
    if n < 2 {
        return factors;
    }
    let mut d = 2;
    while d * d <= n {
        while n.is_multiple_of(d) {
            factors.push(d);
            n /= d;
        }
        d += 1;
    }
    if n > 1 {
        factors.push(n);
    }
    factors
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn primes_below_100() {
        let primes: Vec<u64> = (0..100).filter(|&n| is_prime(n)).collect();
        assert_eq!(
            primes,
            [
                2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79,
                83, 89, 97
            ]
        );
    }

    #[test]
    fn factors() {
        assert_eq!(prime_factors(360), [2, 2, 2, 3, 3, 5]);
        assert_eq!(prime_factors(97), [97]);
        assert_eq!(prime_factors(1), [] as [u64; 0]);
    }

    proptest! {
        #[test]
        fn factors_are_prime_and_multiply_back(n in 2u64..1_000_000) {
            let factors = prime_factors(n);
            prop_assert!(factors.iter().all(|&f| is_prime(f)));
            prop_assert!(factors.is_sorted());
            prop_assert_eq!(factors.iter().product::<u64>(), n);
        }
    }
}
