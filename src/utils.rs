pub fn expand(input: &str) -> Result<String, String> {
    // توسيع المتغيرات داخل الاقتباس المزدوج
    if input.starts_with('"') && input.ends_with('"') {
        let unquoted = &input[1..input.len()-1];
        shellexpand::full(unquoted)
            .map(|s| format!("\"{}\"", s))
            .map_err(|e| format!("Expansion error: {}", e))
    } else {
        shellexpand::full(input)
            .map(|s| s.into_owned())
            .map_err(|e| format!("Expansion error: {}", e))
    }
}
