use crate::utils::expand;
use crate::b_mod; // تحديث المسار

pub fn execute(input: &str) -> Result<(), String> {
    let tokens = parse_input(input)?;
    let expanded_tokens = expand_tokens(&tokens)?;
    b_mod::execute_command(&expanded_tokens) // تحديث المسار
}

fn expand_tokens(tokens: &[String]) -> Result<Vec<String>, String> {
    tokens.iter()
        .map(|token| {
            if token.starts_with('"') && token.ends_with('"') {
                expand(&token[1..token.len()-1])
            } else if token.starts_with('\'') && token.ends_with('\'') {
                Ok(token[1..token.len()-1].to_string())
            } else {
                expand(token)
            }
        })
        .collect()
}

pub fn parse_input(input: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut quote_char = '\0';
    let mut escape_next = false;
    
    for c in input.chars() {
        match c {
            '\\' if !escape_next => escape_next = true,
            '"' | '\'' if !escape_next => {
                if !in_quotes {
                    in_quotes = true;
                    quote_char = c;
                } else if c == quote_char {
                    in_quotes = false;
                } else {
                    current.push(c);
                }
            }
            ' ' | '\t' if !in_quotes && !escape_next => {
                if !current.is_empty() {
                    tokens.push(current);
                    current = String::new();
                }
            }
            _ => {
                if escape_next {
                    current.push('\\');
                }
                current.push(c);
                escape_next = false;
            }
        }
    }
    
    if !current.is_empty() {
        tokens.push(current);
    }
    
    let mut result = Vec::new();
    for token in tokens {
        let mut temp = Vec::new();
        let mut current_part = String::new();
        let mut chars = token.chars().peekable();
        
        while let Some(c) = chars.next() {
            // Handle 24> as a single token
            if c == '2' && chars.peek() == Some(&'4') {
                chars.next(); // Skip '4'
                if chars.peek() == Some(&'>') {
                    chars.next(); // Skip '>'
                    if !current_part.is_empty() {
                        temp.push(current_part);
                        current_part = String::new();
                    }
                    temp.push("24>".to_string());
                    continue;
                } else {
                    current_part.push('2');
                    current_part.push('4');
                    if let Some('>') = chars.peek() {
                        chars.next();
                        current_part.push('>');
                    }
                }
            }
            
            // Handle other symbols
            if c == '>' {
                let mut symbol = c.to_string();
                if let Some('>') = chars.peek() {
                    symbol.push(chars.next().unwrap());
                }
                
                if !current_part.is_empty() {
                    temp.push(current_part);
                    current_part = String::new();
                }
                temp.push(symbol);
                continue;
            }
            
            if c == '1' || c == '2' || c == '&' {
                if let Some('>') = chars.peek() {
                    let mut symbol = c.to_string();
                    symbol.push(chars.next().unwrap());
                    
                    if let Some('>') = chars.peek() {
                        symbol.push(chars.next().unwrap());
                    }
                    
                    if !current_part.is_empty() {
                        temp.push(current_part);
                        current_part = String::new();
                    }
                    temp.push(symbol);
                    continue;
                }
            }
            
            if ";|&<".contains(c) {
                if !current_part.is_empty() {
                    temp.push(current_part);
                    current_part = String::new();
                }
                temp.push(c.to_string());
            } else {
                current_part.push(c);
            }
        }
        
        if !current_part.is_empty() {
            temp.push(current_part);
        }
        
        result.extend(temp);
    }
    
    Ok(result)
}
