#[macro_use]
extern crate async_trait;

#[macro_use]
extern crate anyhow;

use std::time::Duration;
use resource_mesh_portal_api_server::{Portal, PortalMuxer, Message, MuxCall, Router};
use anyhow::Error;
use tokio::sync::mpsc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::error::SendTimeoutError;
use resource_mesh_portal_serde::version::v0_0_1::mesh::inlet::resource::Operation;
use std::sync::Arc;
use resource_mesh_portal_serde::version::v0_0_1::{PrimitiveFrame, mesh};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::convert::{TryInto, TryFrom};
use tokio::net::tcp::{OwnedWriteHalf, OwnedReadHalf};

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
                match std::net::TcpListener::bind(format!("127.0.0.1:{}", self.port)) {
                    Ok(std_listener) => {
                        let listener = TcpListener::from_std(std_listener).unwrap();
                        while let Ok((stream, _)) = listener.accept().await {

                            Self::handle(&muxer, self.server.clone(), stream);


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
        let mut reader = PrimitiveFrameRead::new(reader);
        let mut writer = PrimitiveFrameWrite::new(writer);
        let flavor = reader.read_string().await?;

        // first verify flavor matches
        if flavor != server.flavor() {
            let message = format!("ERROR: flavor does not match.  expected '{}'", server.flavor() );
            writer.write_string(message.clone() ).await?;
            return Err(anyhow!(message));
        }

        writer.write_string( "OK".to_string() ).await?;

        match server.auth(&mut reader, &mut writer).await
        {
            Ok(user) => {
                writer.write_string( "OK".to_string() ).await?;
                let reader = FrameReader::new(reader );
                let writer = FrameWriter::new(writer);
                match server.portal(user.clone(), reader, writer).await {
                    Ok(portal) => {
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
    async fn auth(&self, reader: &mut PrimitiveFrameRead, writer: &mut PrimitiveFrameWrite ) -> Result<String,Error>;
    async fn portal(&self, user: String, reader: FrameReader, writer: FrameWriter ) -> Result<Portal,Error>;
    fn route_to_mesh(&self, message: Message<Operation>);
    fn logger(&self) -> fn(message: &str);
}

pub struct FrameWriter {
    stream: PrimitiveFrameWrite
}

impl FrameWriter {
    pub fn new(stream: PrimitiveFrameWrite) -> Self {
        Self {
            stream
        }
    }

    /*
    pub async fn read( &mut self ) -> Result<mesh::inlet::Frame,Error> {
        let frame = self.stream.read().await?;
        Ok(mesh::inlet::Frame::try_from(frame)?)
    }
     */

    pub async fn write( &mut self, frame: mesh::outlet::Frame ) -> Result<(),Error> {
        let frame = frame.try_into()?;
        self.stream.write(frame).await
    }
}

pub struct FrameReader{
    stream: PrimitiveFrameRead
}

impl FrameReader {
    pub fn new(stream: PrimitiveFrameRead ) -> Self {
        Self {
            stream
        }
    }

    pub async fn read( &mut self ) -> Result<mesh::inlet::Frame,Error> {
        let frame = self.stream.read().await?;
        Ok(mesh::inlet::Frame::try_from(frame)?)
    }

}

pub struct PrimitiveFrameRead {
    read: OwnedReadHalf
}

impl PrimitiveFrameRead {

    pub fn new(read: OwnedReadHalf ) -> Self {
        Self {
           read
        }
    }

    pub async fn read(&mut self) -> Result<PrimitiveFrame,Error> {
        let size = self.read.read_u32().await? as usize;
        let mut vec: Vec<u8> = Vec::with_capacity(size );
        let buf = vec.as_mut_slice();
        self.read.read(buf).await?;
        Result::Ok(PrimitiveFrame {
            size: size as u32,
            data: vec
        })
    }

    pub async fn read_string(&mut self) -> Result<String,Error> {
        let frame = self.read().await?;
        Ok(frame.try_into()?)
    }


}

pub struct PrimitiveFrameWrite {
    write: OwnedWriteHalf
}

impl PrimitiveFrameWrite {

    pub fn new(write: OwnedWriteHalf) -> Self {
        Self {
            write
        }
    }

    /*
    pub async fn read(&mut self) -> Result<PrimitiveFrame,Error> {
        let size = self.write.read_u32().await? as usize;
        let mut vec: Vec<u8> = Vec::with_capacity(size );
        let buf = vec.as_mut_slice();
        self.write.read(buf).await?;
        Result::Ok(PrimitiveFrame {
            size: size as u32,
            data: vec
        })
    }
     */

    pub async fn write( &mut self, frame: PrimitiveFrame ) -> Result<(),Error> {
        self.write.write_u32(frame.size ).await?;
        self.write.write_all(frame.data.as_slice() ).await?;
        Ok(())
    }

    /*
    pub async fn read_string(&mut self) -> Result<String,Error> {
        let frame = self.read().await?;
        Ok(frame.try_into()?)
    }
     */

    pub async fn write_string(&mut self, string: String) -> Result<(),Error> {
        let frame = string.into();
        self.write(frame).await
    }
}

