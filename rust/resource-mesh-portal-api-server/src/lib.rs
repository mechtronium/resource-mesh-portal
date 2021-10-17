pub mod error;

use resource_mesh_portal_serde::{mesh, Address, Key, Status, ExchangeId, ExchangeKind, Identifier, Operation, Log, Signal};
use resource_mesh_portal_serde::config::{Config, BindConfig, Info};
use tokio::sync::mpsc;
use crate::error::Error;
use std::time::{Duration, SystemTime};
use uuid::Uuid;
use std::collections::HashMap;
use std::convert::TryFrom;
use futures::future::select_all;
use futures::FutureExt;

pub enum PortalStatus{
    None,
    Initializing,
    Ready,
    Panic(String)
}

pub fn log( log: Log) {
    match log {
        Log::Info(message) => {
            println!(message);
        }
        Log::Fatal(message) => {
            eprintln!(message);
        }
    }
}


#[derive(Clone)]
pub enum PortalKind {
    Mechtron,
    Portal
}

pub struct Exchange {
    pub id: ExchangeId,
    pub tx: tokio::sync::oneshot::Sender<mesh::enter::Response>,
}

enum PortalCall {
    Frame(mesh::enter::Frame),
    Exchange(Exchange)
}

impl ToString for PortalKind {
    fn to_string(&self) -> String {
        match self {
            PortalKind::Mechtron => "Mechtron".to_string(),
            PortalKind::Portal => "Portal".to_string()
        }
    }
}

pub struct Request {
    pub to: Identifier,
    pub from: Identifier,
    pub operation: Operation,
    pub kind: ExchangeKind,
}

impl Request {
    pub fn from( request: mesh::enter::Request, from: Identifier, to: Identifier ) -> Self {
        Self{
            to,
            from,
            operation: request.operation,
            kind: request.kind
        }
    }
}


pub struct Response {
   pub to: Identifier,
   pub from: Identifier,
   pub exchange_id: ExchangeId,
   pub signal: Signal
}

impl Response {
    pub fn from( response: mesh::exit::Response, from: Identifier, to: Identifier ) -> Self {
        Self{
            to,
            from,
            exchange_id: response.exchange_id,
            signal: response.signal
        }
    }
}



pub struct Portal {
    pub key: Key,
    pub address: Address,
    pub info: Info,
    pub kind: PortalKind,
    exit_tx: mpsc::Sender<mesh::exit::Frame>,
    mux_tx: mpsc::Sender<MuxCall>,
    pub log: fn(log:Log),
    status_tx: tokio::sync::broadcast::Sender<Status>,
    status_rx: tokio::sync::broadcast::Receiver<Status>,
    call_tx: mpsc::Sender<PortalCall>,
    status: PortalStatus,
    pub mux_rx: mpsc::Receiver<MuxCall>,
}

impl Portal {
    pub fn status(&self) -> Status {
        self.status.clone()
    }

    pub fn new(key: Key, address: Address, kind: PortalKind, info: Info, exit_tx: mpsc::Sender<mesh::exit::Frame>, enter_rx: mpsc::Receiver<mesh::enter::Frame>, logger: fn(log:Log) ) -> Self {
        let (mux_tx,mux_rx) = tokio::sync::mpsc::channel(128);
        let (status_tx,status_rx) = tokio::sync::broadcast::channel(1);
        let (call_tx,mut call_rx) = tokio::sync::mpsc::channel(128);
        {
            let command_tx = call_tx.clone();
            let mut enter_rx = enter_rx;
            tokio::spawn(async move {
                while let Option::Some(frame) = enter_rx.recv().await {
                    command_tx.send(PortalCall::Frame(frame)).await.unwrap_or_else(
                        || {
                            logger(Log::Fatal("FATAL: could not send PortalCommand through command_tx channel".to_string()));
                        }
                    );
                }
            });
        }

        {
            let mut exchanges =  HashMap::new();
            let mux_tx= mux_tx.clone();
            let exit_tx = exit_tx.clone();
            let info = info.clone();
            tokio::spawn(async move {
                while let Option::Some(command) = call_rx.recv().await {
                    match command {
                        PortalCall::Frame(frame) => {
                            match frame {
                                mesh::enter::Frame::Log(log) => {
                                    log
                                }
                                mesh::enter::Frame::StartCli(_) => {}
                                mesh::enter::Frame::Command(_) => {}
                                mesh::enter::Frame::EndCli(_) => {}
                                mesh::enter::Frame::Request(request) => {
                                    match &request.kind {
                                        ExchangeKind::Notification => {
                                            for to in &request.to {
                                                let request = Request::from( request.clone(), info.key.clone(), to.clone() );
                                                let result = mux_tx.send_timeout(MuxCall::Request(request), Duration::from_secs(info.config.frame_timeout.clone())).await;
                                                if let Result::Err(err) = result {
                                                    logger(Log::Fatal("FATAL: send timeout error request_tx".to_string()))
                                                }
                                            }
                                        }
                                        ExchangeKind::RequestResponse(exchange_id) => {
                                            if request.to.len() != 1 {
                                                let response = mesh::exit::Response{
                                                    from: info.key.clone(),
                                                    exchange_id: exchange_id.clone(),
                                                    signal: Signal::Error("a RequestResponse message must have one and only one to recipient.".to_string())
                                                };
                                                let result = exit_tx.send_timeout(mesh::exit::Frame::Response(response), Duration::from_secs(info.config.frame_timeout.clone()) ).await;
                                                if let Result::Err(err) = result {
                                                    logger(Log::Fatal("FATAL: frame timeout error exit_tx".to_string()));
                                                }
                                            } else {
                                                let to = request.to.first().expect("expected to identifier").clone();
                                                let request = Request::from( request.clone(), info.key.clone(), to );
                                                let result = mux_tx.send_timeout(MuxCall::Request(request), Duration::from_secs(info.config.frame_timeout.clone())).await;
                                                if let Result::Err(err) = result {
                                                    logger(Log::Fatal("FATAL: frame timeout error request_tx".to_string()));
                                                }
                                            }
                                        }
                                    }

                                }
                                mesh::enter::Frame::Response(response) => {
                                    match exchanges.remove( &response.exchange_id ) {
                                        None => {
                                            logger(Log::Fatal(format!("FATAL: missing request/response exchange id '{}'", response.exchange_id)));
                                        }
                                        Some(tx) => {
                                            tx.send(response);
                                        }
                                    }
                                }
                                mesh::enter::Frame::BinParcel(_) => {}
                                mesh::enter::Frame::SetStatus(status) => {
                                    status_tx.send(status).unwrap_or_default();
                                }
                            }
                        }
                        PortalCall::Exchange(exchange) => {
                            exchanges.insert( exchange.id, exchange.tx );
                        }
                    }
                }
            });

        }

        Self{
            key,
            address,
            kind,
            info,
            call_tx,
            exit_tx,
            status_tx,
            status_rx,
            status: PortalStatus::None,
            log: logger,
            mux_tx: request_tx,
            mux_rx: request_rx
        }
    }

    pub async fn send(&self, frame: mesh::exit::Frame ) -> Result<(),Error> {
        self.exit_tx.send_timeout(frame, self.info.config.frame_timeout.clone() ).await?;
        Ok(())
    }

    pub async fn exchange(&self, request: mesh::exit::Request ) -> Result<mesh::enter::Response,Error> {
        let mut request = request;
        let exchangeId: ExchangeId = Uuid::new_v4().to_string();
        request.kind = ExchangeKind::RequestResponse(exchangeId.clone());
        let (tx,rx) = tokio::sync::oneshot::channel();
        let exchange = Exchange {
            id: exchangeId,
            tx
        };
        self.call_tx.send_timeout(PortalCall::Exchange(exchange), self.info.config.frame_timeout.clone() ).await?;

        Ok(rx.await?)
    }


    pub fn shutdown(&mut self) {
        self.exit_tx.try_send(mesh::exit::Frame::Shutdown).unwrap_or(());
    }

    pub async fn init(&mut self) -> Result<(),Error> {
        if self.status != Status::Unknown {
            return Err(format!("{} has already received the init signal.",self.kind.to_string()).into());
        }

        self.status = PortalStatus::Initializing;

        self.exit_tx.try_send(mesh::exit::Frame::Init(self.info.clone()) )?;
        let mut status_rx = self.status_tx.subscribe();
        let (tx,rx) = tokio::sync::oneshot::channel();
        let config = self.info.config.clone();
        let kind = self.kind.clone();
        tokio::spawn( async move {
            loop {
                let status = if config.init_timeout > 0 {
                    match tokio::time::timeout( Duration::from_secs(config.init_timeout.clone() ), status_rx.recv() ).await {
                        Ok(Ok(status)) => {
                            status
                        }
                        Ok(Result::Err(error)) => {
                            tx.send(Result::Err(format!("ERROR: when waiting for {} status: 'Ready' message: '{}'",kind.to_string(),err.to_string())) );
                            return;
                        }
                        Err(err) => {
                            tx.send(Result::Err(format!("PANIC: {} init timeout after '{}' seconds", kind.to_string(), message).into()) );
                            return;
                        }
                    }
                } else {
                    status_rx.recv().await
                };

                match status_rx.recv().await {
                    Ok(status) => {
                        match status {
                            Status::Ready => {
                                tx.send(Result::Ok(()) );
                            }
                            Status::Panic(message) => {
                                tx.send(Result::Err(format!("PANIC: {} panic on init message: '{}'", kind.to_string(), message).into()) );
                            }
                            _ => {
                                // ignore this status
                            }
                        }
                    }
                    Err(err) => {
                        tx.send(Result::Err(format!("ERROR: when waiting for {} status: 'Ready' message: '{}'", kind.to_string(), err.to_string()).into()));
                    }
                }
            }
        } );

        match rx.await {
            Ok(Ok(_)) => {
                Ok(())
            }
            Ok(Err(err)) => {
                self.status = PortalStatus::Panic(err);
                self.shutdown();
                Err(err.into())
            }
            Err(err) => {
                self.status = PortalStatus::Panic(err.to_string());
                self.shutdown();
                Err(err.to_string().into())
            }
        }
    }
}

pub enum MuxCall {
    Add(Portal),
    Remove(Key),
    Request(Request),
    Response(Response),
}

pub enum Message {
    Request(Request),
    Response(Response)
}

impl Message {
    pub fn to(&self) -> Identifier {
        match self {
            Message::Request(request) => {
                request.to.clone()
            }
            Message::Response(response) => {
                response.to.clone()
            }
        }
    }
}


pub struct PortalMuxer {
    portals: HashMap<Key,Portal>,
    router: fn(message: Message)
}

impl PortalMuxer {
    pub fn new(router: fn(message:Message)) -> mpsc::Sender<MuxCall> {
        let (portal_tx, mut portal_rx) = tokio::sync::mpsc::channel(128);

        let mut muxer = Self {
            portals: HashMap::new(),
            router
        };

        tokio::spawn( async move {

            let mut futures = vec![];
            let mut keys = vec![];

            for (key,portal) in &mut muxer.portals {
                keys.push(key.clone());
                futures.push(portal.mux_rx.recv().boxed() );
            }

            futures.push( portal_rx.recv().boxed() );

            let (call, future_index, _) = select_all(futures).await;

            while let Option::Some(command) = portal_rx.recv().await {
                match command {
                    MuxCall::Add(portal) => {
                        muxer.portals.insert(portal.info.key.clone(), portal );
                    }
                    MuxCall::Remove(key) => {
                        if let Option::Some(mut portal) = muxer.portals.remove(&key) {
                            portal.shutdown();
                        }
                    }
                    MuxCall::Request(request) => {
                        muxer.router(Message::Request(request));
                    }
                    MuxCall::Response(response) => {
                        muxer.router(Message::Response(response));
                    }
                }
            }
        } );

        portal_tx
    }
}