pub fn greeting() -> &'static str {
    "hello"
}

#[cfg(test)]
mod tests {
    use super::greeting;

    #[test]
    fn returns_the_declared_greeting() {
        assert_eq!(greeting(), "hello");
    }
}
