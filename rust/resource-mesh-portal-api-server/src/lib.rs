#[macro_use]
extern crate anyhow;

use std::collections::HashMap;
use std::time::Duration;

use anyhow::Error;
use futures::FutureExt;
use futures::future::select_all;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use resource_mesh_portal_serde::version::v0_0_1::{Address, ExchangeId, ExchangeKind, Identifier, Key, Log, mesh, Signal, Status, Identifiers, IdentifierKind, ExtOperation};
use resource_mesh_portal_serde::version::v0_0_1::config::Info;
use std::sync::Arc;
use resource_mesh_portal_serde::version::v0_0_1::mesh::outlet::Frame;
use tokio::sync::mpsc::error::SendTimeoutError;
use resource_mesh_portal_serde::version::v0_0_1::mesh::inlet::resource::Operation;
use std::prelude::rust_2021::TryInto;

#[derive(Clone,Eq,PartialEq,Hash)]
pub enum PortalStatus{
    None,
    Initializing,
    Ready,
    Panic(String)
}

pub fn log( log: Log) {
    match log {
        Log::Info(message) => {
            println!("{}",message);
        }
        Log::Fatal(message) => {
            eprintln!("{}",message);
        }
        Log::Warn(message) => {
            println!("{}",message);
        }
        Log::Error(message) => {
            eprintln!("{}",message);
        }
    }
}


#[derive(Clone)]
pub enum PortalKind {
    Mechtron,
    Portal
}

#[derive(Debug)]
pub struct Exchange {
    pub id: ExchangeId,
    pub tx: tokio::sync::oneshot::Sender<mesh::inlet::Response>,
}

#[derive(Debug)]
enum PortalCall {
    FrameIn(mesh::inlet::Frame),
    FrameOut(mesh::outlet::Frame),
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


pub struct Request<OPERATION> {
    pub to: Identifier,
    pub from: Identifier,
    pub operation: OPERATION,
    pub kind: ExchangeKind,
}

impl Request<Operation> {
    pub fn from(request: mesh::inlet::Request, from: Identifier, to: Identifier ) -> Self {
        Self{
            to,
            from,
            operation: request.operation,
            kind: request.kind
        }
    }
}

impl Into<mesh::inlet::Request> for Request<Operation> {
    fn into(self) -> mesh::inlet::Request {
        mesh::inlet::Request {
            to: vec![self.to],
            operation: self.operation,
            kind: self.kind
        }
    }
}

impl Into<mesh::outlet::Request> for Request<ExtOperation> {
    fn into(self) -> mesh::outlet::Request {
        mesh::outlet::Request {
            from: self.from,
            operation: self.operation,
            kind: self.kind
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
    pub fn from(response: mesh::outlet::Response, from: Identifier, to: Identifier ) -> Self {
        Self{
            to,
            from,
            exchange_id: response.exchange_id,
            signal: response.signal
        }
    }
}

impl Into<mesh::inlet::Response> for Response {
    fn into(self) -> mesh::inlet::Response {
        mesh::inlet::Response {
            to: self.to,
            exchange_id: self.exchange_id,
            signal: self.signal
        }
    }
}

impl Into<mesh::outlet::Response> for Response {
    fn into(self) -> mesh::outlet::Response {
        mesh::outlet::Response {
            from: self.from,
            exchange_id: self.exchange_id,
            signal: self.signal
        }
    }
}

pub struct Portal {
    pub key: Key,
    pub address: Address,
    pub info: Info,
    pub kind: PortalKind,
    outlet_tx: mpsc::Sender<mesh::outlet::Frame>,
    mux_tx: mpsc::Sender<MuxCall>,
    pub log: fn(log:Log),
    status_tx: tokio::sync::broadcast::Sender<Status>,

    #[allow(dead_code)]
    status_rx: tokio::sync::broadcast::Receiver<Status>,

    call_tx: mpsc::Sender<PortalCall>,
    status: PortalStatus,
    pub mux_rx: mpsc::Receiver<MuxCall>,
}

impl Portal {
    pub fn status(&self) -> PortalStatus {
        self.status.clone()
    }

    pub fn new(key: Key, address: Address, kind: PortalKind, info: Info, outlet_tx: mpsc::Sender<mesh::outlet::Frame>, inlet_rx: mpsc::Receiver<mesh::inlet::Frame>, logger: fn(log:Log) ) -> Self {
        let (mux_tx,mux_rx) = tokio::sync::mpsc::channel(128);
        let (status_tx,status_rx) = tokio::sync::broadcast::channel(1);
        let (call_tx,mut call_rx) = tokio::sync::mpsc::channel(128);
        {
            let command_tx = call_tx.clone();
            let mut inlet_rx = inlet_rx;
            tokio::spawn(async move {
                while let Option::Some(frame) = inlet_rx.recv().await {
                    command_tx.send(PortalCall::FrameIn(frame)).await.unwrap_or_else(
                        |_err| {
                            logger(Log::Fatal("FATAL: could not send PortalCommand through command_tx channel".to_string()));
                        }
                    );
                }
            });
        }

        {
            let mut exchanges:HashMap<ExchangeId,oneshot::Sender<mesh::inlet::Response>> =  HashMap::new();
            let mux_tx= mux_tx.clone();
            let outlet_tx = outlet_tx.clone();
            let info = info.clone();
            let status_tx = status_tx.clone();
            tokio::spawn(async move {
                while let Option::Some(command) = call_rx.recv().await {
                    match command {
                        PortalCall::FrameIn(frame) => {
                            match frame {
                                mesh::inlet::Frame::Log(log) => {
                                    (logger)(log);
                                }
                                mesh::inlet::Frame::Command(_) => {}
                                mesh::inlet::Frame::Request(request) => {
                                    match &request.kind {
                                        ExchangeKind::None=> {
                                            logger(Log::Fatal("FATAL: received request with an invalid 'ExchangeKind::None'".to_string()))
                                        }
                                        ExchangeKind::Notification => {
                                            for to in &request.to {
                                                let request = Request::from( request.clone(), Identifier::Key(info.key.clone()), to.clone() );
                                                let result = mux_tx.send_timeout(MuxCall::MessageIn(Message::Request(request)), Duration::from_secs(info.config.frame_timeout.clone())).await;
                                                if let Result::Err(_err) = result {
                                                    logger(Log::Fatal("FATAL: send timeout error request_tx".to_string()))
                                                }
                                            }
                                        }
                                        ExchangeKind::RequestResponse(exchange_id) => {
                                            if request.to.len() != 1 {
                                                let response = mesh::outlet::Response{
                                                    from: Identifier::Key(info.key.clone()),
                                                    exchange_id: exchange_id.clone(),
                                                    signal: Signal::Error("a RequestResponse message must have one and only one to recipient.".to_string())
                                                };
                                                let result = outlet_tx.send_timeout(mesh::outlet::Frame::Response(response), Duration::from_secs(info.config.frame_timeout.clone()) ).await;
                                                if let Result::Err(_err) = result {
                                                    logger(Log::Fatal("FATAL: frame timeout error exit_tx".to_string()));
                                                }
                                            } else {
                                                let to = request.to.first().expect("expected to identifier").clone();
                                                let request = Request::from( request.clone(), Identifier::Key(info.key.clone()), to );
                                                let result = mux_tx.send_timeout(MuxCall::MessageIn(Message::Request(request)), Duration::from_secs(info.config.frame_timeout.clone())).await;
                                                if let Result::Err(_err) = result {
                                                    logger(Log::Fatal("FATAL: frame timeout error request_tx".to_string()));
                                                }
                                            }
                                        }
                                    }

                                }
                                mesh::inlet::Frame::Response(response) => {
                                    match exchanges.remove( &response.exchange_id ) {
                                        None => {
                                            logger(Log::Fatal(format!("FATAL: missing request/response exchange id '{}'", response.exchange_id)));
                                        }
                                        Some(tx) => {
                                            let tx = tx;
                                            tx.send(response).expect("ability to send response");
                                        }
                                    }
                                }
                                mesh::inlet::Frame::BinParcel(_) => {}
                                mesh::inlet::Frame::Status(status) => {
                                    status_tx.send(status).unwrap_or_default();
                                }
                            }
                        }
                        PortalCall::Exchange(exchange) => {
                            exchanges.insert( exchange.id, exchange.tx );
                        }
                        PortalCall::FrameOut(frame) => {
                            match outlet_tx.send_timeout(frame, Duration::from_secs(info.config.frame_timeout )).await {
                                Ok(_) => {}
                                Err(err) => {
                                    logger(Log::Fatal("FATAL: frame timeout error outlet_tx".to_string()));
                                }
                            }
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
            outlet_tx,
            status_tx,
            status_rx,
            status: PortalStatus::None,
            log: logger,
            mux_tx,
            mux_rx
        }
    }

    pub async fn send(&self, frame: mesh::outlet::Frame ) -> Result<(), Error> {
        self.outlet_tx.send_timeout(frame, Duration::from_secs( self.info.config.frame_timeout.clone() ) ).await?;
        Ok(())
    }

    pub async fn exchange(&self, request: mesh::outlet::Request ) -> Result<mesh::inlet::Response, Error> {
        let mut request = request;
        let exchange_id: ExchangeId = Uuid::new_v4().to_string();
        request.kind = ExchangeKind::RequestResponse(exchange_id.clone());
        let (tx,rx) = tokio::sync::oneshot::channel();
        let exchange = Exchange {
            id: exchange_id,
            tx
        };
        self.call_tx.send_timeout(PortalCall::Exchange(exchange), Duration::from_secs(self.info.config.frame_timeout.clone()) ).await?;

        Ok(rx.await?)
    }


    pub fn shutdown(&mut self) {
        self.outlet_tx.try_send(mesh::outlet::Frame::Shutdown).unwrap_or(());
    }

    pub async fn init(&mut self) -> Result<(), Error> {
        if self.status != PortalStatus::None {
            let message = format!("{} has already received the init signal.",self.kind.to_string());
            return Err(anyhow!(message));
        }

        self.status = PortalStatus::Initializing;

        self.outlet_tx.try_send(mesh::outlet::Frame::Init(self.info.clone()) )?;
        let mut status_rx = self.status_tx.subscribe();
        let (tx,rx) = tokio::sync::oneshot::channel();
        let config = self.info.config.clone();
        let kind = self.kind.clone();
        tokio::spawn( async move {
            loop {
                let _status = if config.init_timeout > 0 {
                    match tokio::time::timeout( Duration::from_secs(config.init_timeout.clone() ), status_rx.recv() ).await {
                        Ok(Ok(status)) => {
                            status
                        }
                        Ok(Result::Err(err)) => {
                            tx.send(Result::Err(format!("ERROR: when waiting for {} status: 'Ready' message: '{}'",kind.to_string(),err.to_string())) ).expect("ability to send error");
                            break;
                        }
                        Err(_err) => {
                            tx.send(Result::Err(format!("PANIC: {} init timeout after '{}' seconds", kind.to_string(), config.init_timeout.clone() ).into()) ).expect("ability to send error");
                            break;
                        }
                    }
                } else {
                    match status_rx.recv().await {
                        Ok(status) => status,
                        Err(err) => {
                            tx.send(Result::Err(format!("ERROR: when waiting for {} status: 'Ready' message: '{}'",kind.to_string(),err.to_string())) ).expect("ability to send error");
                            break;
                        }
                    }
                };

                match status_rx.recv().await {
                    Ok(status) => {
                        match status {
                            Status::Ready => {
                                tx.send(Result::Ok(()) ).expect("ability to send ok");
                                break;
                            }
                            Status::Panic(message) => {
                                tx.send(Result::Err(format!("PANIC: {} panic on init message: '{}'", kind.to_string(), message).into()) ).expect("ability to send error");
                                break;
                            }
                            _ => {
                                // ignore this status
                            }
                        }
                    }
                    Err(err) => {
                        tx.send(Result::Err(format!("ERROR: when waiting for {} status: 'Ready' message: '{}'", kind.to_string(), err.to_string()).into())).expect("ability to send error");
                        break;
                    }
                }
            }
        } );

        match rx.await {
            Ok(Ok(_)) => {
                Ok(())
            }
            Ok(Err(err)) => {
                self.status = PortalStatus::Panic(err.clone());
                self.shutdown();
                Err(anyhow!(err))
            }
            Err(err) => {
                self.status = PortalStatus::Panic(err.to_string());
                self.shutdown();
                Err(anyhow!(err))
            }
        }
    }
}

pub enum MuxCall {
    Add(Portal),
    Remove(Identifier),
    MessageIn(Message<Operation>),
    MessageOut(Message<ExtOperation>)
}

pub enum Message<OPERATION> {
    Request(Request<OPERATION>),
    Response(Response)
}

impl <OPERATION> Message<OPERATION> {
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


pub trait Router: Send+Sync {
    fn route( &self, message: Message<Operation>);
    fn logger( &self, message: String ) {
        println!("{}", message );
    }
}

pub struct PortalMuxer {
    portals: HashMap<Identifier,Portal>,
    router: Arc<dyn Router>,
    address_to_key: HashMap<Address,Key>,
    key_to_address: HashMap<Key,Address>,
}

impl PortalMuxer {
    pub fn new( router: Arc<dyn Router> ) -> mpsc::Sender<MuxCall> {
        let (portal_tx, mut portal_rx) = tokio::sync::mpsc::channel(128);

        let mut muxer = Self {
            portals: HashMap::new(),
            address_to_key: HashMap::new(),
            key_to_address: HashMap::new(),
            router,
        };

        tokio::spawn( async move {

            let mut ids = vec![];
            let mut futures = vec![];

            for (key,portal) in &mut muxer.portals {
                futures.push( portal.mux_rx.recv().boxed() );
                ids.push(key.clone());
            }

            futures.push( portal_rx.recv().boxed() );

            let (call, future_index, _) = select_all(futures).await;

            match call {
                None => {
                    if future_index >= ids.len() {
                        // shutdown
                        return;
                    } else {
                        let key = ids.get(future_index).expect("expected key");
                        if let Option::Some(mut portal) = muxer.portals.remove(key) {
                            portal.shutdown();
                        }
                    }
                }
                Some(call) => {
                    match call {
                        MuxCall::Add(portal) => {
                            muxer.key_to_address.insert(portal.info.key.clone(), portal.info.address.clone() );
                            muxer.address_to_key.insert(portal.info.address.clone(), portal.info.key.clone() );
                            muxer.portals.insert(Identifier::Key(portal.info.key.clone()), portal );
                        }
                        MuxCall::Remove(id) => {
                            let key = match &id {
                                Identifier::Key(key) => {
                                    Option::Some(key)
                                }
                                Identifier::Address(address) => {
                                    muxer.address_to_key.get(address)
                                }
                            };

                            if let Option::Some( key ) = key {
                                if let Option::Some(mut portal) = muxer.portals.remove(&Identifier::Key(key.clone()) ) {
                                    muxer.key_to_address.remove(&portal.info.key);
                                    muxer.address_to_key.remove(&portal.info.address);

                                    portal.shutdown();
                                }
                            }
                        }
                        MuxCall::MessageIn(message) => {
                            muxer.router.route( message );
                        }
                        MuxCall::MessageOut(message) => {
                            match muxer.get_portal(&message.to())
                            {
                                Some(portal) => {
                                    match message {
                                        Message::Request(request) => {
                                            portal.call_tx.try_send( PortalCall::FrameOut( mesh::outlet::Frame::Request(request.into())));
                                        }
                                        Message::Response(response) => {
                                            portal.call_tx.try_send( PortalCall::FrameOut( mesh::outlet::Frame::Response(response.into())));
                                        }
                                    }
                                }
                                None => {}
                            }
                        }
                    }
                }
            }

        } );

        portal_tx
    }

    fn get_portal( &self, id: &Identifier ) -> Option<&Portal> {
        match id {
            Identifier::Key(key) => {
                self.portals.get(id)
            }
            Identifier::Address(address) => {
                let key = self.address_to_key.get(address );
                match key {
                    Some(key) => {
                        self.portals.get(&Identifier::Key(key.clone()))
                    }
                    None => {
                        Option::None
                    }
                }
            }
        }

    }
}