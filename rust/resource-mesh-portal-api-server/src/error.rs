

pub struct PortalError {
    pub message: String
}

impl From<String> for PortalError {
    fn from(message: String) -> Self {
        Self{
            message
        }
    }
}