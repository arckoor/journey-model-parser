#[derive(Debug)]
pub struct ParseError {
    msg: String,
}

impl ParseError {
    pub fn new(msg: &str) -> Self {
        ParseError {
            msg: msg.to_string(),
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}
