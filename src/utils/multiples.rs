pub fn smallest_multiple_greater_than_or_equal_to(value: f32, multiple: f32) -> f32 {
    let remainder = value % multiple;

    if remainder == 0.0 {
        return value;
    }

    if remainder.is_sign_negative() {
        return value - remainder;
    }
    value + multiple - remainder
}

pub fn largest_multiple_less_than_or_equal_to(value: f32, multiple: f32) -> f32 {
    let remainder = value % multiple;

    if remainder.is_sign_negative() {
        return value - multiple - remainder;
    }
    value - remainder
}

#[cfg(test)]
mod test {
    #[test]
    fn test_smallest_multiple_greater_than_or_equal_to() {
        assert_eq!(
            super::smallest_multiple_greater_than_or_equal_to(5.0, 3.0),
            6.0
        );
        assert_eq!(
            super::smallest_multiple_greater_than_or_equal_to(6.0, 3.0),
            6.0
        );
        assert_eq!(
            super::smallest_multiple_greater_than_or_equal_to(7.0, 3.0),
            9.0
        );
        assert_eq!(
            super::smallest_multiple_greater_than_or_equal_to(8.0, 3.0),
            9.0
        );
        assert_eq!(
            super::smallest_multiple_greater_than_or_equal_to(9.0, 3.0),
            9.0
        );

        assert_eq!(
            super::smallest_multiple_greater_than_or_equal_to(112.9, 10.0),
            120.0
        );
        assert_eq!(
            super::smallest_multiple_greater_than_or_equal_to(-112.9, 10.0),
            -110.0
        );
    }

    #[test]
    fn test_largest_multiple_less_than_or_equal_to() {
        assert_eq!(super::largest_multiple_less_than_or_equal_to(5.0, 3.0), 3.0);
        assert_eq!(super::largest_multiple_less_than_or_equal_to(6.0, 3.0), 6.0);
        assert_eq!(super::largest_multiple_less_than_or_equal_to(7.0, 3.0), 6.0);
        assert_eq!(super::largest_multiple_less_than_or_equal_to(8.0, 3.0), 6.0);
        assert_eq!(super::largest_multiple_less_than_or_equal_to(9.0, 3.0), 9.0);

        assert_eq!(
            super::largest_multiple_less_than_or_equal_to(112.9, 10.0),
            110.0
        );
        assert_eq!(
            super::largest_multiple_less_than_or_equal_to(-112.9, 10.0),
            -120.0
        );
    }
}
