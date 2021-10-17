

pub struct Error {
    pub message: String
}

impl From<String> for Error {
    fn from(message: String) -> Self {
        Self{
            message
        }
    }
}