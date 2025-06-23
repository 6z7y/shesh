#[derive(Debug, PartialEq, Clone)]
pub enum Symbol {
    OutputRedirect,
    AppendRedirect,
    InputRedirect,
    Pipe,
}

// New type definition to simplify code
pub type RedirectionResult = Result<(Vec<String>, Option<String>, Option<String>, bool), String>;

pub fn to_symbol(s: &str) -> Option<Symbol> {
    match s {
        ">" => Some(Symbol::OutputRedirect),
        ">>" => Some(Symbol::AppendRedirect),
        "<" => Some(Symbol::InputRedirect),
        "|" => Some(Symbol::Pipe),
        _ => None,
    }
}

pub fn handle_symbol(symbol: &Symbol, _args: &[String]) -> Result<(), String> {
    match symbol {
        Symbol::OutputRedirect | Symbol::AppendRedirect | Symbol::InputRedirect => {
            Err("Redirection symbols must be used with a command".to_string())
        }
        Symbol::Pipe => {
            Err("Pipe symbol requires commands on both sides".to_string())
        }
    }
}

pub fn find_redirections(args: &[String]) -> RedirectionResult {
    let mut clean_args = Vec::new();
    let mut input_file = None;
    let mut output_file = None;
    let mut append = false;
    let mut skip_next = false;
    
    for (i, arg) in args.iter().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }
        
        // Remove quotes for symbol detection
        let trimmed_arg = arg.trim_matches('"');
        
        match to_symbol(trimmed_arg) {
            Some(Symbol::InputRedirect) => {
                if let Some(file) = args.get(i + 1) {
                    input_file = Some(file.trim_matches('"').to_string());
                    skip_next = true;
                } else {
                    return Err("Input file missing for redirection".to_string());
                }
            }
            Some(Symbol::OutputRedirect) => {
                if let Some(file) = args.get(i + 1) {
                    output_file = Some(file.trim_matches('"').to_string());
                    skip_next = true;
                    append = false;
                } else {
                    return Err("Output file missing for redirection".to_string());
                }
            }
            Some(Symbol::AppendRedirect) => {
                if let Some(file) = args.get(i + 1) {
                    output_file = Some(file.trim_matches('"').to_string());
                    skip_next = true;
                    append = true;
                } else {
                    return Err("Output file missing for append".to_string());
                }
            }
            _ => clean_args.push(arg.clone()),
        }
    }
    
    Ok((clean_args, input_file, output_file, append))
}
