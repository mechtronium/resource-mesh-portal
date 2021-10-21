#[macro_use]
extern crate async_trait;


#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    use tokio::net::TcpStream;
    use tokio::runtime::Runtime;
    use tokio::sync::mpsc;

    use resource_mesh_portal_api_server::{Message, Portal, PortalMuxer, Router};
    use resource_mesh_portal_serde::version::v0_0_1::{Identifier, Log};
    use resource_mesh_portal_serde::version::v0_0_1::config::{Config, Info, PortalKind};
    use resource_mesh_portal_serde::version::v0_0_1::mesh::inlet::resource::Operation;
    use resource_mesh_portal_serde::version::v0_0_1::resource::Archetype;
    use resource_mesh_portal_tcp_server::{ PortalServer, PortalTcpServer, PrimitiveFrameRead, PrimitiveFrameWrite, FrameWriter, FrameReader};

    #[test]
    fn server_up() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            PortalTcpServer::new( 45678, Box::new( TestPortalServer::new() ));
        });
    }

    pub struct TestRouter {

    }

    impl Router for TestRouter {
        fn route(&self, message: Message<Operation>) {
            todo!()
        }
    }

    pub struct TestPortalServer {
        pub atomic: AtomicU32
    }

    impl TestPortalServer {
        pub fn new() -> Self {
            Self {
                atomic: AtomicU32::new(0)
            }
        }
    }

    fn test_logger( message: &str ) {
        println!("{}", message );
    }

    #[async_trait]
    impl PortalServer for TestPortalServer {
        fn flavor(&self) -> String {
            "test".to_string()
        }

        async fn auth(&self, reader: &mut PrimitiveFrameRead, writer: &mut PrimitiveFrameWrite) -> Result<String, anyhow::Error> {
            let username = reader.read_string().await?;
            Ok(username)
        }

        async fn portal(&self, user: String, reader: FrameReader, writer: FrameWriter ) -> Result<Portal, anyhow::Error> {
            let index = self.atomic.fetch_add(1,Ordering::Relaxed);
            let key = format!("({})",index);
            let address = format!("portal-{}",index);

            let info = Info {
                key,
                address,
                parent: Identifier::Address("parent".to_string()),
                archetype: Archetype {
                    kind: "Portal".to_string(),
                    specific: None,
                    config_src: None
                },
                config: Default::default(),
                ext_config: None,
                kind: PortalKind::Portal
            };

            let (outlet_tx,mut outlet_rx) = mpsc::channel(128);
            let (inlet_tx,inlet_rx) = mpsc::channel(128);

            fn logger( log: Log ) {
                println!("{}", log.to_string() );
            }

            let portal = Portal::new(info, outlet_tx, inlet_rx, logger );

            let mut reader = reader;
            {
                let logger = self.logger();
                tokio::spawn(async move {
                    while let Result::Ok(frame) = reader.read().await {
                        let result = inlet_tx.try_send(frame);
                        if result.is_err() {
                            (logger)("FATAL: cannot send frame to portal inlet_tx");
                            return;
                        }
                    }
                });
            }

            let mut writer= writer;
            {
                let logger = self.logger();
                tokio::spawn(async move {
                    while let Option::Some(frame) = outlet_rx.recv().await {
                        let result = writer.write(frame).await;
                        if result.is_err() {
                            (logger)("FATAL: cannot wwrite frame to frame writer");
                            return;
                        }
                    }
                });
            }

            Ok(portal)
        }

        fn route_to_mesh(&self, message: Message<Operation>) {
            todo!()
        }

        fn logger(&self) -> fn(&str) {
            test_logger
        }
    }

}

