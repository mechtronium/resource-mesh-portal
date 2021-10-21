#[cfg(test)]
mod tests {
    use resource_mesh_portal_api_server::{PortalMuxer, Message, Router};
    use tokio::runtime::Runtime;
    use resource_mesh_portal_serde::version::v0_0_1::mesh::inlet::resource::Operation;
    use std::sync::Arc;

    #[test]
    fn server_up() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let _muxer = PortalMuxer::new(Arc::new(TestRouter{}) );
            println!("muxer started.");
        });
    }

    pub struct TestRouter {

    }

    impl Router for TestRouter {
        fn route(&self, message: Message<Operation>) {
            todo!()
        }
    }
}

