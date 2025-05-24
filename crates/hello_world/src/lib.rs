pub fn say_hello() -> String {
    "Hello, World!".to_string()
}

pub fn say_hello_to(name: &str) -> String {
    format!("Hello, {}!", name)
}

pub fn run_hello_world() {
    println!("{}", say_hello());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_say_hello() {
        assert_eq!(say_hello(), "Hello, World!");
    }

    #[test]
    fn test_say_hello_to() {
        assert_eq!(say_hello_to("Rust"), "Hello, Rust!");
        assert_eq!(say_hello_to("世界"), "Hello, 世界!");
    }
}