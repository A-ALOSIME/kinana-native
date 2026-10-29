use rand::rngs::OsRng;
use rand::Rng;

pub fn generate_password(
    length: usize,
    use_symbols: bool,
    use_numbers: bool,
    use_uppercase: bool,
) -> String {
    let lower = b"abcdefghijklmnopqrstuvwxyz";
    let upper = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let numbers = b"0123456789";
    let symbols = b"!@#$%^&*()-_=+[]{}|;:,.<>?";

    let mut pool = Vec::with_capacity(100);
    pool.extend_from_slice(lower);

    let mut guaranteed = Vec::with_capacity(4);
    let mut rng = OsRng;

    // Guarantee lowercase
    guaranteed.push(lower[rng.gen_range(0..lower.len())] as char);

    if use_uppercase {
        pool.extend_from_slice(upper);
        guaranteed.push(upper[rng.gen_range(0..upper.len())] as char);
    }
    if use_numbers {
        pool.extend_from_slice(numbers);
        guaranteed.push(numbers[rng.gen_range(0..numbers.len())] as char);
    }
    if use_symbols {
        pool.extend_from_slice(symbols);
        guaranteed.push(symbols[rng.gen_range(0..symbols.len())] as char);
    }

    let target_len = length.max(guaranteed.len()).min(128);
    let mut result: Vec<char> = guaranteed;

    while result.len() < target_len {
        let idx = rng.gen_range(0..pool.len());
        result.push(pool[idx] as char);
    }

    // Shuffle the result securely with Fisher-Yates
    for i in (1..result.len()).rev() {
        let j = rng.gen_range(0..=i);
        result.swap(i, j);
    }

    result.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_password_length() {
        let pwd = generate_password(20, true, true, true);
        assert_eq!(pwd.len(), 20);
    }

    #[test]
    fn test_generate_password_guarantees() {
        for _ in 0..20 {
            let pwd = generate_password(16, true, true, true);
            assert!(pwd.chars().any(|c| c.is_ascii_lowercase()));
            assert!(pwd.chars().any(|c| c.is_ascii_uppercase()));
            assert!(pwd.chars().any(|c| c.is_ascii_digit()));
            assert!(pwd.chars().any(|c| "!@#$%^&*()-_=+[]{}|;:,.<>?".contains(c)));
        }
    }

    #[test]
    fn test_generate_password_no_symbols() {
        let pwd = generate_password(16, false, true, true);
        assert!(!pwd.chars().any(|c| "!@#$%^&*()-_=+[]{}|;:,.<>?".contains(c)));
    }
}
