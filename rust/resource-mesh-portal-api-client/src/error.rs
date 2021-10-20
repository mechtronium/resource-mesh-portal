use tokio::time::error::Elapsed;
use tokio::sync::oneshot::error::RecvError;
use tokio::sync::mpsc::error::SendTimeoutError;
use resource_mesh_portal_serde::mesh::outlet::Frame;
use std::fmt::Display;

pub struct PortalError {
    pub message: String
}



/*
impl <T> From<T> for Error where T: ToString {
    fn from(error: T ) -> Self {
        Self {
            message: error.to_string()
        }
    }
}

 */

impl <E> From<E> for PortalError where E: Display {
    fn from(d: E) -> Self {
        Self{
            message: d.to_string()
        }
    }
}

