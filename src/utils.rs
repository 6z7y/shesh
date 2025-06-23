//! Utility functions

pub fn expand(input: &str) -> Result<String, String> {
    // If the string is quoted, return it as is without quotes
    if (input.starts_with('"') && input.ends_with('"')) ||
       (input.starts_with('\'') && input.ends_with('\'')) {
        return Ok(input[1..input.len()-1].to_string());
    }
    
    // Expand variables and tilde
    shellexpand::full(input)
        .map(|expanded| {
            let result = expanded.into_owned();
            
            // Remove extra quotes if present
            if (result.starts_with('"') && result.ends_with('"')) ||
               (result.starts_with('\'') && result.ends_with('\'')) {
                result[1..result.len()-1].to_string()
            } else {
                result
            }
        })
        .map_err(|e| format!("Expansion error: {}", e))
}
