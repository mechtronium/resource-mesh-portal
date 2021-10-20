#[cfg(test)]
mod tests {
    use resource_mesh_portal_api_server::{PortalMuxer, Message, MuxCall};
    use tokio::runtime::Runtime;

    #[test]
    fn server_up() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let mut muxer = PortalMuxer::new(router);
            println!("muxer started.");
        });
    }

    pub fn router( message: Message ) {

    }
}

