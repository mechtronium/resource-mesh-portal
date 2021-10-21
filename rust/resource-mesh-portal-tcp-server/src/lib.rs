#[macro_use]
extern crate async_trait;

#[macro_use]
extern crate anyhow;

use std::convert::{TryFrom, TryInto};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::SendTimeoutError;

use resource_mesh_portal_api_server::{Message, MuxCall, Portal, PortalMuxer, Router};
use resource_mesh_portal_serde::version::v0_0_1::{mesh, PrimitiveFrame, Log};
use resource_mesh_portal_serde::version::v0_0_1::mesh::inlet::resource::Operation;
use resource_mesh_portal_tcp_common::{FrameReader, FrameWriter, PrimitiveFrameReader, PrimitiveFrameWriter};
use resource_mesh_portal_serde::version::v0_0_1::config::Info;

pub enum PortalServerCall {
    InjectMessage(Message<Operation>)
}

pub struct PortalTcpServer {
    pub port: usize,
    pub server: Arc<dyn PortalServer>,
}

impl PortalTcpServer {

    pub fn new(port: usize, server: Box<dyn PortalServer>) -> Self {
        Self {
            port,
            server: server.into()
        }
    }


    pub fn start(self) -> mpsc::Sender<PortalServerCall> {
        let (server_tx,mut server_rx) = mpsc::channel(128);
        let router = Box::new(RouterProxy::new(self.server.clone()));
        let muxer = PortalMuxer::new(router);
        {
            let muxer = muxer.clone();
            tokio::spawn(async move {
                while let Option::Some(call) = server_rx.recv().await {
                    match call {
                        PortalServerCall::InjectMessage(_) => {}
                    }
                }
            });
        }

        {
            tokio::spawn(async move {
println!("SERVER: BINDING to 127.0.0.1... ");
                match std::net::TcpListener::bind(format!("127.0.0.1:{}", self.port)) {
                    Ok(std_listener) => {
                        let listener = TcpListener::from_std(std_listener).unwrap();
println!("SERVER: waiting for connection ...  ");
                        while let Ok((stream, _)) = listener.accept().await {
println!("SERVER: accepted connection");

                            Self::handle(&muxer, self.server.clone(), stream ).await;


                            tokio::time::sleep(Duration::from_secs(0)).await;
                        }
                    }
                    Err(error) => {
                        (self.server.logger())(format!("FATAL: could not setup TcpListener {}", error).as_str());
                    }
                }
            });
        }
        server_tx
    }

    async fn handle( muxer: &mpsc::Sender<MuxCall>, server: Arc<dyn PortalServer>, stream: TcpStream ) -> Result<(),Error> {
        let (reader, writer) = stream.into_split();
        let mut reader = PrimitiveFrameReader::new(reader);
        let mut writer = PrimitiveFrameWriter::new(writer);
println!("SERVER: READING flavor...");

        let flavor = reader.read_string().await?;
println!("SERVER: flavor is '{}'", flavor );

/*if true {
    return Ok(());
}

 */

        // first verify flavor matches
        if flavor != server.flavor() {
            let message = format!("ERROR: flavor does not match.  expected '{}'", server.flavor() );

println!("SERVER: MESSAGE: {}", message );
println!("SERVER: message frame.size: {}", PrimitiveFrame::from( message.clone() ).size()  );
println!("SERVER: bad flavor, writing message: {}",message  );
writer.enabled = true;
/*if true {
    return Ok(())
}*/
            writer.write_string(message.clone() ).await?;
            tokio::time::sleep(Duration::from_secs(0)).await;



            return Err(anyhow!(message));

        } else {
            println!("accepted flavor");
        }


        writer.write_string( "Ok".to_string() ).await?;
        tokio::time::sleep(Duration::from_secs(0)).await;

        match server.auth(&mut reader, &mut writer).await
        {
            Ok(user) => {
                tokio::time::sleep(Duration::from_secs(0)).await;
                writer.write_string( "Ok".to_string() ).await?;

                let mut reader : FrameReader<mesh::inlet::Frame> = FrameReader::new(reader );
                let mut writer : FrameWriter<mesh::outlet::Frame>  = FrameWriter::new(writer );

if true {
return Ok(());
}

                match server.info(user.clone() ).await {
                    Ok(info) => {
                        tokio::time::sleep(Duration::from_secs(0)).await;

                        let (outlet_tx,mut outlet_rx) = mpsc::channel(128);
                        let (inlet_tx,inlet_rx) = mpsc::channel(128);

                        fn logger( log: Log ) {
                            println!("{}", log.to_string() );
                        }

                        let portal = Portal::new(info, outlet_tx, inlet_rx, logger );

                        let mut reader = reader;
                        {
                            let logger = server.logger();
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
                            let logger = server.logger();
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

                        match muxer.try_send(MuxCall::Add(portal)) {
                            Err(err) => {
                                (server.logger())(err.to_string().as_str())
                            }
                            _ => {
                            }
                        }
                    }
                    Err(err) => {
                        (server.logger())(format!("portal creation error: {}", err.to_string()).as_str())
                    }
                }
            }
            Err(err) => {
                let message = format!("ERROR: authorization failed: {}", err.to_string());
                (server.logger())(message.as_str());
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

