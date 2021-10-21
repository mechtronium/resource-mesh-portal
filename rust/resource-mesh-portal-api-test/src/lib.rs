#[macro_use]
extern crate async_trait;


#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    use tokio::net::TcpStream;
    use tokio::runtime::{Runtime, Builder};
    use tokio::sync::mpsc;

    use resource_mesh_portal_api_server::{Message, Portal, PortalMuxer, Router};
    use resource_mesh_portal_serde::version::v0_0_1::{Identifier, Log};
    use resource_mesh_portal_serde::version::v0_0_1::config::{Config, Info, PortalKind};
    use resource_mesh_portal_serde::version::v0_0_1::mesh::inlet::resource::Operation;
    use resource_mesh_portal_serde::version::v0_0_1::resource::Archetype;
    use resource_mesh_portal_tcp_server::{PortalServer, PortalTcpServer};
    use resource_mesh_portal_tcp_common::{PrimitiveFrameReader, PrimitiveFrameWriter, FrameReader, FrameWriter};
    use resource_mesh_portal_tcp_client::{PortalTcpClient, PortalClient};
    use anyhow::Error;
    use resource_mesh_portal_api_client::{PortalSkel, InletApi, PortalCtrl};
    use tokio::time::Duration;
    use tokio::io;
    use std::io::Write;
    use tokio::io::AsyncWriteExt;
    use std::thread;

    #[tokio::test]
    async fn server_up() {
//        let rt = Builder::new_current_thread().enable_all().build().unwrap();



        thread::spawn( || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                launch_server().await;
            } );
        });
        thread::sleep( Duration::from_secs(1));
        thread::spawn( || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {launch_client().await; } );
        });
//        let mut stdout = io::stdout();
//        stdout.write_all(b"Hello world!").await;
        tokio::time::sleep(Duration::from_secs(5)).await;
println!("got to the end...");
    }

    pub async fn launch_server() {
        {
            let mut stdout = io::stdout();
 //           stdout.write_all(b"AWAY WE GO!").await;
            println!("AWAY WE GO2 ");
            println!("AWAY WE GO3 ");
                println!("spawning something...");
                let port = 32344;
                let server = PortalTcpServer::new( port , Box::new( TestPortalServer::new() ));
                let server_caller = server.start();

                println!("server should be up...");

            tokio::time::sleep(Duration::from_secs(5)).await;

        }
    }

    pub async fn launch_client() {
        let port = 32344;
        {
           eprintln!("client spawn");
            let client = Box::new(TestPortalClient::new("scott".to_string()) );
            //thread::sleep( Duration::from_secs(5 ));
            let client = PortalTcpClient::new(format!("localhost:{}",port), client ).await;
            match client {
                Ok(_) => {
                    println!("client should be connected...");
                }
                _ => {
                    eprintln!("client ERROR" );
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
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

