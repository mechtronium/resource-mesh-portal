#[macro_use]
extern crate async_trait;

#[macro_use]
extern crate anyhow;



use resource_mesh_portal_tcp_common::{PrimitiveFrameReader, PrimitiveFrameWriter, FrameWriter, FrameReader};
use anyhow::Error;
use resource_mesh_portal_api_client::{Portal, PortalCtrl, PortalSkel, InletApi, Inlet };
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use resource_mesh_portal_serde::version::latest::portal;
use resource_mesh_portal_serde::version::latest::log::Log;
use tokio::sync::mpsc::error::TrySendError;
use resource_mesh_portal_serde::version;
use std::thread;
use tokio::time::Duration;
use resource_mesh_portal_serde::version::latest::portal::{outlet, inlet};


pub struct PortalTcpClient {
    pub host: String,
    pub portal: Arc<Portal>
}

impl PortalTcpClient {

    pub async fn new( host: String, client: Box<dyn PortalClient> ) -> Result<Self,Error> {

        let stream = TcpStream::connect(host.clone()).await?;

        let (reader,writer) = stream.into_split();
        let mut reader = PrimitiveFrameReader::new(reader);
        let mut writer = PrimitiveFrameWriter::new(writer);

        writer.write_string(client.flavor()).await?;

        let result = reader.read_string().await?;


        if result != "Ok" {
            let message = format!("FLAVOR MATCH FAILED: {}",result);
            (client.logger())(message.as_str());
            return Err(anyhow!(message));
        }

        client.auth(&mut reader, &mut writer).await?;

        let result = reader.read_string().await?;

        if result != "Ok" {
            let message = format!("AUTH FAILED: {}",result);
            (client.logger())(message.as_str());
            return Err(anyhow!(message));
        }

        let mut reader : FrameReader<outlet::Frame> = FrameReader::new(reader );
        let mut writer : FrameWriter<inlet::Frame>  = FrameWriter::new(writer );


        let (inlet_tx, mut inlet_rx) = mpsc::channel(1024 );
        let (outlet_tx, mut outlet_rx) = mpsc::channel(1024 );

        {
            let logger = client.logger();
            tokio::spawn(async move {
                while let Option::Some(frame) = inlet_rx.recv().await {
                    match writer.write(frame).await {
                        Ok(_) => {}
                        Err(err) => {
                            (logger)("FATAL: writer disconnected");
                            break;
                        }
                    }
                }
            });
        }


        let inlet = Box::new(TcpInlet{
          sender: inlet_tx,
           logger: client.logger()
        });


        if let outlet::Frame::Init(info) = reader.read( ).await?  {
            let portal = Portal::new(info, inlet, client.portal_ctrl_factory(), client.logger()).await?;


            {
                let logger = client.logger();
                tokio::spawn(async move {
                    while let Result::Ok(frame) = reader.read().await {
                        match outlet_tx.try_send( frame ) {
                            Result::Ok(_) => {}
                            Result::Err(err) => {
                                (logger)("FATAL: reader disconnected");
                                break;
                            }
                        }
                    }
                });
            }


            return Ok(Self {
                host,
                portal
            });
        } else {
            let message = "expected portal info.".to_string();
            (client.logger())(message.as_str());
            return Err(anyhow!(message));
        }
    }
}

#[async_trait]
pub trait PortalClient: Send+Sync {
    fn flavor(&self) -> String;
    async fn auth( &self, reader: & mut PrimitiveFrameReader, writer: & mut PrimitiveFrameWriter ) -> Result<(),Error>;
    fn portal_ctrl_factory(&self)->fn( skel: PortalSkel) -> Box<dyn PortalCtrl>;
    fn logger(&self) -> fn(message: &str);
}

struct TcpInlet {
    pub sender: mpsc::Sender<inlet::Frame>,
    pub logger: fn( message: &str )
}

impl Inlet for TcpInlet {
    fn send_frame(&self, frame: inlet::Frame) {
        match self.sender.try_send(frame)
        {
            Ok(_) => {}
            Err(err) => {
                (self.logger)(format!("ERROR: frame failed to send to client inlet").as_str())
            }
        }
    }
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
