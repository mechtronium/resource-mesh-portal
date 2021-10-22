#[macro_use]
extern crate async_trait;

#[macro_use]
extern crate anyhow;

#[macro_use]
extern crate strum_macros;


use std::convert::{TryFrom, TryInto};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::{mpsc, oneshot, broadcast, Mutex};
use tokio::sync::mpsc::error::SendTimeoutError;

use resource_mesh_portal_api_server::{Message, MuxCall, Portal, PortalMuxer, Router};
use resource_mesh_portal_serde::version::v0_0_1::{mesh, PrimitiveFrame, Log, Status,CloseReason};
use resource_mesh_portal_serde::version::v0_0_1::mesh::inlet::resource::Operation;
use resource_mesh_portal_tcp_common::{FrameReader, FrameWriter, PrimitiveFrameReader, PrimitiveFrameWriter};
use resource_mesh_portal_serde::version::v0_0_1::config::Info;

#[derive(Clone,strum_macros::Display)]
pub enum Event {
    Status(Status),
    ClientConnected,
    FlavorNegotiation(EventResult<String>),
    Authorization(EventResult<String>),
    Info(EventResult<Info>),
    Shutdown,
}

#[derive(Clone)]
pub enum EventResult<E>{
    Ok(E),
    Err(String)
}

pub enum PortalServerCall {
    ListenEvents(oneshot::Sender<broadcast::Receiver<Event>>),
    InjectMessage(Message<Operation>),
    Shutdown
}

struct Alive {
    pub alive: bool
}

impl Alive {
    pub fn new() -> Self {
        Self {
            alive: true
        }
    }
}

pub struct PortalTcpServer {
    pub port: usize,
    pub server: Arc<dyn PortalServer>,
    pub broadcaster_tx: broadcast::Sender<Event>
}

impl PortalTcpServer {

    pub fn new(port: usize, server: Box<dyn PortalServer>) -> Self {
        let (broadcaster_tx,_) = broadcast::channel(32);
        Self {
            port,
            server: server.into(),
            broadcaster_tx
        }
    }


    pub fn start(self) -> mpsc::Sender<PortalServerCall> {
        self.broadcaster_tx.send( Event::Status(Status::Initializing) ).unwrap_or_default();

        let alive = Arc::new( Mutex::new( Alive::new() ));
        let (server_tx,mut server_rx) = mpsc::channel(128);
        let router = Box::new(RouterProxy::new(self.server.clone()));
        let port = self.port.clone();
        let muxer = PortalMuxer::new(router);
println!("SERVER: start");
        {
            let alive = alive.clone();
            let broadcaster_tx = self.broadcaster_tx.clone();
            let muxer = muxer.clone();
println!("SERVER: pre-spawn");
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(0)).await;
println!("SERVER: call loop");
                while let Option::Some(call) = server_rx.recv().await {
                    match call {
                        PortalServerCall::InjectMessage(_) => {}
                        PortalServerCall::ListenEvents(tx) => {
println!("SERVER: RECEIVED request for broadcast_Tx");
                            tx.send( broadcaster_tx.subscribe() );
                        },
                        PortalServerCall::Shutdown  => {
                            broadcaster_tx.send(Event::Shutdown).unwrap_or_default();
                            alive.lock().await.alive = false;
                            match std::net::TcpStream::connect(format!("localhost:{}", port)) {
                                Ok(_) => {}
                                Err(_) => {}
                            }
                            return;
                        }
                    }
                }
            });
        }


        {
            let alive = alive.clone();
            tokio::spawn(async move {

                let addr = format!("localhost:{}", self.port);
                match std::net::TcpListener::bind(addr.clone()) {


                    Ok(std_listener) => {
                        tokio::time::sleep(Duration::from_secs(0)).await;
                        let listener = TcpListener::from_std(std_listener).unwrap();
                        self.broadcaster_tx.send( Event::Status(Status::Ready) ).unwrap_or_default();
                        tokio::time::sleep(Duration::from_secs(0)).await;
println!("STARTED LISTENING addr {}", addr);
                        while let Ok((stream, _)) = listener.accept().await {
                            {
println!("CONNECTION:w");
                                if !alive.lock().await.alive.clone() {
                                    self.broadcaster_tx.send( Event::Status(Status::Done) ).unwrap_or_default();
                                    (self.server.logger())("server reached final shutdown");
                                    return;
                                }
                            }
                            self.broadcaster_tx.send( Event::ClientConnected ).unwrap_or_default();
                            (&self).handle(&muxer, stream ).await;
                        }
println!("~~ completed listenering  ~~~");
                    }
                    Err(error) => {
                        let message = format!("FATAL: could not setup TcpListener {}", error);
                        (self.server.logger())(message.as_str());
                        self.broadcaster_tx.send( Event::Status(Status::Panic(message)) ).unwrap_or_default();
                    }
                }
            });
        }
        server_tx
    }

    async fn handle( &self, muxer: &mpsc::Sender<MuxCall>, stream: TcpStream ) -> Result<(),Error> {
        let (reader, writer) = stream.into_split();
        let mut reader = PrimitiveFrameReader::new(reader);
        let mut writer = PrimitiveFrameWriter::new(writer);

        let flavor = reader.read_string().await?;

        // first verify flavor matches
        if flavor != self.server.flavor() {
            let message = format!("ERROR: flavor does not match.  expected '{}'", self.server.flavor() );

            writer.write_string(message.clone() ).await?;
            tokio::time::sleep(Duration::from_secs(0)).await;

            self.broadcaster_tx.send( Event::FlavorNegotiation(EventResult::Err(message.clone()))).unwrap_or_default();
            return Err(anyhow!(message));
        } else {
            self.broadcaster_tx.send( Event::FlavorNegotiation(EventResult::Ok(self.server.flavor()))).unwrap_or_default();
        }


        writer.write_string( "Ok".to_string() ).await?;
        tokio::time::sleep(Duration::from_secs(0)).await;

        match self.server.auth(&mut reader, &mut writer).await
        {
            Ok(user) => {
                self.broadcaster_tx.send( Event::Authorization(EventResult::Ok(user.clone()))).unwrap_or_default();
                tokio::time::sleep(Duration::from_secs(0)).await;
                writer.write_string( "Ok".to_string() ).await?;

                let mut reader : FrameReader<mesh::inlet::Frame> = FrameReader::new(reader );
                let mut writer : FrameWriter<mesh::outlet::Frame>  = FrameWriter::new(writer );

                match self.server.info(user.clone() ).await {
                    Ok(info) => {

                        self.broadcaster_tx.send( Event::Info(EventResult::Ok(info.clone()))).unwrap_or_default();
                        tokio::time::sleep(Duration::from_secs(0)).await;

                        let (outlet_tx,mut outlet_rx) = mpsc::channel(128);
                        let (inlet_tx,inlet_rx) = mpsc::channel(128);

                        fn logger( log: Log ) {
                            println!("{}", log.to_string() );
                        }

                        let portal = Portal::new(info.clone(), outlet_tx, inlet_rx, logger );

                        let mut reader = reader;
                        {
                            let logger = self.server.logger();
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
                            let logger = self.server.logger();
                            tokio::spawn(async move {
                                while let Option::Some(frame) = outlet_rx.recv().await {
                                    let result = writer.write(frame).await;
                                    if result.is_err() {
                                        (logger)("FATAL: cannot write to frame writer");
                                        return;
                                    }
                                }
                            });
                        }

                        match muxer.send_timeout(MuxCall::Add(portal),Duration::from_secs(info.config.frame_timeout.clone()), ).await {
                            Err(err) => {
                                let message = err.to_string();
                                (self.server.logger())(message.as_str());
                                self.broadcaster_tx.send( Event::Info(EventResult::Err(message.clone()))).unwrap_or_default();
                            }
                            _ => {}
                        }
                    }
                    Err(err) => {
                        let message = format!("ERROR: portal creation error: {}", err.to_string());
                        (self.server.logger())(message.as_str());
                        self.broadcaster_tx.send( Event::Info(EventResult::Err(message.clone()))).unwrap_or_default();
                        writer.close( CloseReason::Error(message) ).await;
                    }
                }
            }
            Err(err) => {
                let message = format!("ERROR: authorization failed: {}", err.to_string());
                (self.server.logger())(message.as_str());
                self.broadcaster_tx.send( Event::Authorization(EventResult::Err(message.clone()))).unwrap_or_default();
                writer.write_string( message ).await?;
            }
        }
        Ok(())
    }
}

pub struct RouterProxy {
    pub server: Arc<dyn PortalServer>
}

impl RouterProxy {
    pub fn new( server: Arc<dyn PortalServer> ) -> Self {
        Self {
            server
        }
    }
}

impl Router for RouterProxy {
    fn route(&self, message: Message<Operation>) {
        self.server.route_to_mesh(message);
    }

    fn logger( &self, message: &str ) {
        (self.server.logger())(message);
    }
}

#[async_trait]
pub trait PortalServer: Sync+Send {
    fn flavor(&self) -> String;
    async fn auth(&self, reader: &mut PrimitiveFrameReader, writer: &mut PrimitiveFrameWriter) -> Result<String,Error>;
    fn route_to_mesh(&self, message: Message<Operation>);
    fn logger(&self) -> fn(message: &str);
    async fn info(&self, user: String ) -> Result<Info,Error>;
}

