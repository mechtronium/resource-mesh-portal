#[macro_use]
extern crate anyhow;

use std::time::Duration;
use resource_mesh_portal_api_server::{Portal, PortalMuxer, Message, MuxCall};
use anyhow::Error;
use tokio::sync::mpsc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::error::SendTimeoutError;

pub enum PortalServerCall {
    InjectMessage(Message)
}

pub struct PortalTcpServer {
    pub port: usize,
    pub server: Box<dyn PortalServer>,
}

impl PortalTcpServer {

    pub fn new(port: usize, server: Box<dyn PortalServer>) -> Self {
        Self {
            port,
            server
        }
    }

    pub fn start(self) -> mpsc::Sender<PortalServerCall> {
        let (server_tx,mut server_rx) = mpsc::channel(128);
        let muxer = PortalMuxer::new(self.server.router);
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
                            match self.server.auth(&stream)
                            {
                                Ok(user) => {
                                    match self.server.portal(user, stream) {
                                        Ok(portal) => {
                                            match muxer.try_send(MuxCall::Add(portal)) {
                                                Err(err) => {
                                                    self.server.logger(err.to_string().as_str())
                                                }
                                                _ => {}
                                            }
                                        }
                                        Err(err) => {
                                            self.server.logger(format!("portal creation error: {}", err.to_string()).as_str())
                                        }
                                    }
                                }
                                Err(err) => {
                                    self.server.logger(format!("authorization failed: {}", err.to_string()).as_str())
                                }
                            }

                            tokio::time::sleep(Duration::from_secs(0)).await;
                        }
                    }
                    Err(error) => {
                        self.server.logger(format!("FATAL: could not setup TcpListener {}", error).as_str());
                    }
                }
            });
        }
        server_tx
    }
}

pub trait PortalServer {
    fn flavor(&self) -> String;
    fn auth( &self, stream: &TcpStream ) -> Result<String,Error>;
    fn portal( &self, user: String, stream: TcpStream ) -> Result<Portal,Error>;
    fn logger( &self, message: &str );
    fn router( &self, message: Message );
}

