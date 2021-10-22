#[macro_use]
extern crate async_trait;


#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    use tokio::net::TcpStream;
    use tokio::runtime::{Runtime, Builder};
    use tokio::sync::{mpsc, oneshot};

    use resource_mesh_portal_api_server::{Message, Portal, PortalMuxer, Router};
    use resource_mesh_portal_serde::version::v0_0_1::{Identifier, Log};
    use resource_mesh_portal_serde::version::v0_0_1::config::{Config, Info, PortalKind};
    use resource_mesh_portal_serde::version::v0_0_1::mesh::inlet::resource::Operation;
    use resource_mesh_portal_serde::version::v0_0_1::resource::Archetype;
    use resource_mesh_portal_tcp_server::{PortalServer, PortalTcpServer, Event, PortalServerCall};
    use resource_mesh_portal_tcp_common::{PrimitiveFrameReader, PrimitiveFrameWriter, FrameReader, FrameWriter};
    use resource_mesh_portal_tcp_client::{PortalTcpClient, PortalClient};
    use anyhow::Error;
    use resource_mesh_portal_api_client::{PortalSkel, InletApi, PortalCtrl};
    use tokio::time::Duration;
    use tokio::io;
    use std::io::Write;
    use tokio::io::AsyncWriteExt;
    use std::thread;
    use tokio::sync::broadcast::Receiver;
    use tokio::sync::oneshot::error::RecvError;

    #[tokio::test]
    async fn server_up() -> Result<(),Error> {
//        let rt = Builder::new_current_thread().enable_all().build().unwrap();


        let (server_tx,server_rx) = tokio::sync::oneshot::channel();
        println!("got to HERE...");
        tokio::spawn( async move
            {
                let mut server_caller :tokio::sync::mpsc::Sender<PortalServerCall>= server_rx.await.unwrap();
                let (tx,rx) = tokio::sync::oneshot::channel();
                server_caller.send(PortalServerCall::ListenEvents(tx)).await;
                match rx.await {
                    Ok(mut server_events) => {
                        println!("waiting for events...");
                        while let Result::Ok(event) = server_events.recv().await {
                            println!("event: {}",event.to_string());
                        }
                    }
                    Err(_) => {
                        println!("broadcast shutdown");
                    }
                }
            });

        thread::spawn( || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                launch_server(server_tx).await;
                println!("SERVER: block_on done.");
            } );
        });


        thread::spawn( || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                match launch_client().await {
                    Ok(_) => {
                        println!("CLIENT: terminated normally" );
                    }
                    Err(error) => {
                        println!("CLIENT ERROR: {}", error.to_string())
                    }
                }
            } );
        });

        tokio::time::sleep(Duration::from_secs(3)).await;


        tokio::time::sleep(Duration::from_secs(3)).await;

        thread::sleep( Duration::from_secs(3));


        println!("got to the end...");
        Ok(())
    }

    pub async fn launch_server(server_tx: tokio::sync::oneshot::Sender<mpsc::Sender<PortalServerCall>>) {
            let port = 32355;
            let server = PortalTcpServer::new( port , Box::new( TestPortalServer::new() ));
            let server_caller = server.start();
            server_tx.send(server_caller).unwrap_or_default();
            println!("launch_server complete...");
        tokio::time::sleep(Duration::from_secs(3)).await;
    }

    pub async fn launch_client() -> Result<(),Error> {
        let port = 32355;
            let client = Box::new(TestPortalClient::new("scott".to_string()) );
            //thread::sleep( Duration::from_secs(5 ));
            PortalTcpClient::new(format!("localhost:{}",port), client ).await?;
            Ok(())
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

        async fn auth(&self, reader: &mut PrimitiveFrameReader, writer: &mut PrimitiveFrameWriter) -> Result<String, anyhow::Error> {
            let username = reader.read_string().await?;
            tokio::time::sleep(Duration::from_secs(0)).await;
            Ok(username)
        }

        async fn info(&self, user: String) -> Result<Info, anyhow::Error> {
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

            Ok(info)
        }

        fn route_to_mesh(&self, message: Message<Operation>) {
            todo!()
        }

        fn logger(&self) -> fn(&str) {
            test_logger
        }
    }


    pub struct TestPortalClient {
        pub user: String
    }

    impl TestPortalClient {
        pub fn new( user: String ) -> Self {
            Self {
                user
            }
        }
    }

    #[async_trait]
    impl PortalClient for TestPortalClient {
        fn flavor(&self) -> String {
           return "test".to_string()
        }

        async fn auth(&self, reader: &mut PrimitiveFrameReader, writer: &mut PrimitiveFrameWriter) -> Result<(), Error> {
            writer.write_string(self.user.clone() ).await?;
            Ok(())
        }

        fn portal_ctrl_factory(&self) -> fn(Arc<PortalSkel>, InletApi) -> Box<dyn PortalCtrl> {
            return test_portal_ctrl_factory
        }

        fn logger(&self) -> fn(m:&str) {
            fn logger( message: &str ) {
                 println!("{}",message);
            }
            logger
        }
    }




    fn test_portal_ctrl_factory( skel: Arc<PortalSkel>, inlet: InletApi ) -> Box< dyn PortalCtrl> {
        Box::new(TestPortalCtrl {} )
    }


    pub struct TestPortalCtrl {

    }

    #[async_trait]
    impl PortalCtrl for TestPortalCtrl {

        async fn init(&mut self) -> Result<(), Error>{
            println!("TestPortalCtrl.init()");
            Ok(())
        }
    }


}

