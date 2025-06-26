use reedline::{ValidationResult, Validator};
pub struct MyValidator;

impl Validator for MyValidator {
    fn validate(&self, line: &str) -> ValidationResult {
        if line.ends_with('\\') {
            ValidationResult::Incomplete
        } else {
            ValidationResult::Complete
        }
    }
}
