pub fn divide(a: i64, b: i64) -> Result<i64, &'static str> {
    if b == 0 {
        return Err("b is zero");
    }
    Ok(a / b)
}

pub fn classify(n: i64) -> &'static str {
    if n < 0 {
        "neg"
    } else if n == 0 {
        "zero"
    } else {
        "pos"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_divide() {
        assert_eq!(divide(6, 3), Ok(2));
        assert_eq!(divide(1, 0), Err("b is zero"));
    }

    #[test]
    fn test_classify() {
        assert_eq!(classify(-1), "neg");
        assert_eq!(classify(0), "zero");
        assert_eq!(classify(5), "pos");
    }
}
