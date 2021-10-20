#[cfg(test)]
mod tests {
    use resource_mesh_portal_api_server::{PortalMuxer, Message, MuxCall};

    #[test]
    fn server_up() {
        let mut muxer = PortalMuxer::new(router);
        println!("muxer started.");
    }

    pub fn router( message: Message ) {

    }
}

