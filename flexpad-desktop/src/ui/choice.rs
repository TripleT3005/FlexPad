use std::fmt;

// Choice

#[derive(Debug, Clone)]
pub struct Choice<T> {
    pub value: T,
    pub label: String,
}

impl<T> Choice<T> {
    pub fn new(value: T, label: String) -> Self {
        Self { value, label }
    }
}

impl<T: PartialEq> PartialEq for Choice<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T> fmt::Display for Choice<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choices_compare_by_value_and_show_their_label() {
        let first = Choice::new(1, "One".to_string());
        assert_eq!(first, Choice::new(1, "Eins".to_string()));
        assert_ne!(first, Choice::new(2, "One".to_string()));
        assert_eq!(first.to_string(), "One");
    }
}
