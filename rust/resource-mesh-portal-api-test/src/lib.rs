#[macro_use]
extern crate async_trait;

#[macro_use]
extern crate anyhow;

#[macro_use]
extern crate lazy_static;


#[cfg(test)]
mod tests {

lazy_static! {
    static ref GLOBAL_TX : tokio::sync::broadcast::Sender<GlobalEvent> = {
        tokio::sync::broadcast::channel(128).0
    };

}

    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    use tokio::net::TcpStream;
    use tokio::runtime::{Builder, Runtime};
    use tokio::sync::{mpsc, oneshot};

    use anyhow::Error;
    use resource_mesh_portal_api_client::{InletApi, PortalCtrl, PortalSkel, client, PortCtrl};
    use resource_mesh_portal_api_server::{Message, MuxCall, Portal, PortalMuxer, Router};

    use resource_mesh_portal_tcp_client::{PortalClient, PortalTcpClient};
    use resource_mesh_portal_tcp_common::{
        FrameReader, FrameWriter, PrimitiveFrameReader, PrimitiveFrameWriter,
    };
    use resource_mesh_portal_tcp_server::{Call, Event, PortalServer, PortalTcpServer};
    use std::collections::HashMap;
    use std::convert::TryInto;
    use std::io::Write;
    use std::thread;
    use tokio::io;
    use tokio::io::AsyncWriteExt;
    use tokio::sync::broadcast::Receiver;
    use tokio::sync::mpsc::Sender;
    use tokio::sync::oneshot::error::RecvError;
    use tokio::time::Duration;
    use resource_mesh_portal_serde::version::latest::resource::{Status, ResourceStub, Selector};
    use resource_mesh_portal_serde::version::latest::operation::{Operation, ResourceOperation, ExtOperation, PortOperation};
    use resource_mesh_portal_serde::version::latest::config::{Info, PortalKind};
    use resource_mesh_portal_serde::version::latest::id::Identifier;
    use resource_mesh_portal_serde::version::latest::messaging::ExchangeKind;
    use resource_mesh_portal_serde::version::latest::delivery::{Entity, Payload, ResponseEntity};
    use resource_mesh_portal_serde::version::latest::resource::Archetype;
    use resource_mesh_portal_serde::version::latest::delivery::ResourceEntity;
    use resource_mesh_portal_serde::version::latest::portal::inlet;

    #[derive(Clone)]
    pub enum GlobalEvent {
        Finished(String),
        Fail(String),
        Shutdown
    }

    #[tokio::test]
    async fn server_up() -> Result<(), Error> {
        let port = 32355;
        let server = PortalTcpServer::new(port, Box::new(TestPortalServer::new()));
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        {
            let server = server.clone();
            tokio::spawn(async move {
                let mut finished_count = 0;
                let mut GLOBAL_RX = GLOBAL_TX.subscribe();
                while let Result::Ok(event) = GLOBAL_RX.recv().await {
                    match event {
                        GlobalEvent::Finished(_) => {
                            finished_count = finished_count+1;
                        }
                        GlobalEvent::Fail(_) => {
                            finished_count = finished_count+1;
                        }
                        GlobalEvent::Shutdown => {
                            server.send( Call::Shutdown ).await.unwrap_or_default();
                            return;
                        }
                    }
                    if finished_count >= 2 {
                       server.send( Call::Shutdown ).await.unwrap_or_default();
                       return;
                    }
                }
            });
        }

        {
            let server = server.clone();
            tokio::spawn(async move {
                let (listen_tx, listen_rx) = tokio::sync::oneshot::channel();
                server.send(Call::ListenEvents(listen_tx)).await;
                let mut broadcast_rx = listen_rx.await.unwrap();
                while let Result::Ok(event) = broadcast_rx.recv().await {
                    if let Event::Status(status) = &event {
                        println!("event: Status({})", status.to_string());
                    } else {
                        println!("event: {}", event.to_string());
                    }
                    match event {
                        Event::Status(Status::Done) => {
                            shutdown_tx.send(());
                            return;
                        }
                        Event::Status(Status::Panic(error)) => {
                            eprintln!("PANIC: {}", error);
                            shutdown_tx.send(());
                            return;
                        }

                        _ => {}
                    }
                }
            });
        }

        let client1 = Box::new(TestPortalClient::new("scott".to_string()));
        let client2 = Box::new(TestPortalClient::new("fred".to_string()));

        let client1 = PortalTcpClient::new(format!("localhost:{}", port), client1).await?;
        let client2 = PortalTcpClient::new(format!("localhost:{}", port), client2).await?;

        tokio::spawn( async move {
            tokio::time::sleep( Duration::from_secs( 5 ) ).await;
            GLOBAL_TX.send( GlobalEvent::Shutdown ).unwrap_or_default();
        });

        shutdown_rx.await;

        println!("got to the end...");
        Ok(())
    }

    pub struct TestRouter {}

    impl Router for TestRouter {
        fn route(&self, message: Message<Operation>) {
            todo!()
        }
    }

    pub struct TestPortalServer {
        pub atomic: AtomicU32,
    }

    impl TestPortalServer {
        pub fn new() -> Self {
            Self {
                atomic: AtomicU32::new(0),
            }
        }
    }

    fn test_logger(message: &str) {
        println!("{}", message);
    }

    #[async_trait]
    impl PortalServer for TestPortalServer {
        fn flavor(&self) -> String {
            "test".to_string()
        }

        async fn auth(
            &self,
            reader: &mut PrimitiveFrameReader,
            writer: &mut PrimitiveFrameWriter,
        ) -> Result<String, anyhow::Error> {
            let username = reader.read_string().await?;
            tokio::time::sleep(Duration::from_secs(0)).await;
            Ok(username)
        }

        async fn info(&self, user: String) -> Result<Info, anyhow::Error> {
            let index = self.atomic.fetch_add(1, Ordering::Relaxed);
            let key = format!("({})", index);
            let address = format!("portal-{}", index);

            let info = Info {
                key,
                address,
                owner: user,
                parent: Identifier::Address("parent".to_string()),
                archetype: Archetype {
                    kind: "Portal".to_string(),
                    specific: None,
                    config_src: None,
                },
                config: Default::default(),
                ext_config: None,
                kind: PortalKind::Portal,
            };

            Ok(info)
        }

        fn logger(&self) -> fn(&str) {
            test_logger
        }

        fn router_factory(&self, mux_tx: Sender<MuxCall>) -> Box<dyn Router> {
            Box::new(InYourFaceRouter { mux_tx })
        }
    }

    pub struct InYourFaceRouter {
        mux_tx: Sender<MuxCall>,
    }
    impl Router for InYourFaceRouter {
        fn route(&self, message: Message<Operation>) {
            let mux_tx = self.mux_tx.clone();
            tokio::spawn(async move {
                match message {
                    Message::Request(request) => {
                        match &request.operation {
                            Operation::Resource(operation) => {
                                match operation {
                                    ResourceOperation::Select(selector) => {
                                        if let ExchangeKind::RequestResponse(exchange_id) =
                                            &request.kind
                                        {
                                            let (tx, rx) = oneshot::channel();

                                            // this is clearly not an implementation of the Selector
                                            // in this case it will always select everything from the Muxer
                                            fn selector(info: &Info) -> bool {
                                                true
                                            }
                                            let selector = MuxCall::Select { selector, tx };
                                            mux_tx.try_send(selector);
                                            let infos = rx.await.expect("expected infos");
                                            let resources: Vec<ResourceStub> = infos
                                                .iter()
                                                .map(|i| ResourceStub {
                                                    id: Identifier::Address(i.address.clone()),
                                                    key: i.key.clone(),
                                                    address: i.address.clone(),
                                                    archetype: i.archetype.clone(),
                                                })
                                                .collect();
                                            let signal = ResponseEntity::Ok(Entity::Resource(
                                                ResourceEntity::Stubs(resources),
                                            ));
                                            let response =
                                                resource_mesh_portal_api_server::Response {
                                                    to: request.from.clone(),
                                                    from: request.to.clone(),
                                                    exchange_id: exchange_id.clone(),
                                                    signal,
                                                };
                                            mux_tx
                                                .try_send(MuxCall::MessageOut(Message::Response(
                                                    response,
                                                )))
                                                .unwrap_or_default();
                                        }
                                    }
                                    _ => match &request.kind {
                                        ExchangeKind::RequestResponse(exchange_id) => {
                                            let response = resource_mesh_portal_api_server::Response {
                                                to: request.from.clone(),
                                                from: request.to.clone(),
                                                exchange_id: exchange_id.clone(),
                                                signal: ResponseEntity::Error("this is a primitive router that cannot handle resource commands other than Select".to_string())
                                            };
                                            mux_tx.try_send(MuxCall::MessageOut(
                                                Message::Response(response),
                                            ));
                                        }
                                        _ => {}
                                    },
                                }
                            }
                            Operation::Ext(_) => {
                                // since we are not connected to a mesh all inbound messages are just sent back to the outbound
                                mux_tx.try_send(MuxCall::MessageOut(Message::Request(
                                    request.try_into().unwrap(),
                                )));
                            }
                        }
                    }
                    Message::Response(response) => {
                        // since we are not connected to a mesh all inbound messages are just sent back to the outbound
                        mux_tx.try_send(MuxCall::MessageOut(Message::Response(response)));
                    }
                }
            });
        }
    }
    pub struct TestPortalClient {
        pub user: String,
    }

    impl TestPortalClient {
        pub fn new(user: String) -> Self {
            Self { user }
        }
    }

    #[async_trait]
    impl PortalClient for TestPortalClient {
        fn flavor(&self) -> String {
            return "test".to_string();
        }

        async fn auth(
            &self,
            reader: &mut PrimitiveFrameReader,
            writer: &mut PrimitiveFrameWriter,
        ) -> Result<(), Error> {
            writer.write_string(self.user.clone()).await?;
            Ok(())
        }

        fn portal_ctrl_factory(&self) -> fn(PortalSkel) -> Box<dyn PortalCtrl> {
            return friendly_portal_ctrl_factory;
        }

        fn logger(&self) -> fn(m: &str) {
            fn logger(message: &str) {
                println!("{}", message);
            }
            logger
        }
    }

    fn friendly_portal_ctrl_factory(skel: PortalSkel) -> Box<dyn PortalCtrl> {
        Box::new(FriendlyPortalCtrl { skel })
    }

    pub struct FriendlyPortalCtrl {
        pub skel: PortalSkel
    }

    #[async_trait]
    impl PortalCtrl for FriendlyPortalCtrl {
        async fn init(&mut self) -> Result<(), Error> {
            println!("FriendlyPortalCtrl.init()");
            // wait just a bit to make sure everyone got chance to be in the muxer
            tokio::time::sleep(Duration::from_millis(50));

            let mut request = inlet::Request::new(Operation::Resource(
                ResourceOperation::Select(Selector::new()),
            ));
            request.to.push(self.skel.info.parent.clone());

println!("FriendlyPortalCtrl::exchange...");
            match self.skel.api().exchange(request).await {
                Ok(response) => match response.signal {
                    ResponseEntity::Ok(Entity::Resource(ResourceEntity::Stubs(resources))) => {
println!("FriendlyPortalCtrl::Ok");
                        for resource in resources {
                            if resource.key != self.skel.info.key {
                                (self.skel.logger)(format!(
                                    "INFO: found resource: {}",
                                    resource.address
                                ).as_str());
                                let mut request = inlet::Request::new(Operation::Ext(
                                    ExtOperation::Port(PortOperation {
                                        port: "greet".to_string(),
                                        entity: Entity::Payload(Payload::Text(format!(
                                            "Hello, my name is '{}' and I live at '{}'",
                                            self.skel.info.owner, self.skel.info.address
                                        ))),
                                    }),
                                ));
                                let result = self.skel.api().exchange(request).await;
                                match result {
                                    Ok(response) => {
                                        match &response.signal {
                                            ResponseEntity::Ok(Entity::Payload(Payload::Text(response))) => {
                                                println!("got response: {}", response );
                                                GLOBAL_TX.send(GlobalEvent::Finished(self.skel.info.owner.clone()));
                                            }
                                            _ => {
                                                GLOBAL_TX.send(GlobalEvent::Fail(self.skel.info.owner.clone()));
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        GLOBAL_TX.send(GlobalEvent::Fail(self.skel.info.owner.clone()));
                                    }
                                }
                            }
                        }
                    }
                    _ => (self.skel.logger)("ERROR: unexpected response!"),
                },
                Err(_) => {}
            }

            Ok(())
        }

        fn ports(&self) -> HashMap<String,Box<dyn PortCtrl>> {

            struct GreetPort {
                skel: PortalSkel
            }

            #[async_trait]
            impl PortCtrl for GreetPort {
                async fn request( &self, request: client::Request<PortOperation> ) -> Result<Option<ResponseEntity>,Error>{
                    match &request.entity {
                        Entity::Payload(Payload::Text(text)) => Ok(Option::Some(ResponseEntity::Ok(
                            Entity::Payload(Payload::Text("Hello, <username>".to_string())),
                        ))),
                        _ => Err(anyhow!("unexpected request entity")),
                    }
                }
            }

            impl GreetPort {
                pub fn new( skel: PortalSkel ) -> Self {
                    Self {
                        skel
                    }
                }
            }

            let mut ports = HashMap::new();
            let port : Box<dyn PortCtrl> = Box::new(GreetPort::new(self.skel.clone()));
            ports.insert( "greet".to_string(), port );
            ports
        }

        /*
        fn ports( &self, ) -> HashMap< String, fn( request: client::Request<PortOperation>, ) -> Result<Option<ResponseEntity>, Error>> {

             fn greet( request: client::Request<PortOperation>, ) -> Result<Option<ResponseEntity>, Error> {
                match &request.entity {
                    Entity::Payload(Payload::Text(text)) => Ok(Option::Some(ResponseEntity::Ok(
                        Entity::Payload(Payload::Text("Hello, <username>".to_string())),
                    ))),
                    _ => Err(anyhow!("unexpected request entity")),
                }
            }

            let mut ports = HashMap::new();
            ports.insert("greet".to_string(), greet );
            ports
        }

         */
    }
}
