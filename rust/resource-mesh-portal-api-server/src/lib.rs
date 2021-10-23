#[macro_use]
extern crate anyhow;

use std::collections::HashMap;
use std::future::Future;
use std::prelude::rust_2021::TryInto;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Error;
use futures::future::select_all;
use futures::FutureExt;
use tokio::sync::{mpsc, oneshot};
use tokio::sync::mpsc::error::{SendError, SendTimeoutError};
use uuid::Uuid;

use resource_mesh_portal_serde::message as request_message;
use resource_mesh_portal_serde::version::latest::config::Info;
use resource_mesh_portal_serde::version::latest::delivery::ResponseEntity;
use resource_mesh_portal_serde::version::latest::frame::CloseReason;
use resource_mesh_portal_serde::version::latest::id::{Address, Identifier, Key};
use resource_mesh_portal_serde::version::latest::log::Log;
use resource_mesh_portal_serde::version::latest::messaging::{ExchangeId, ExchangeKind};
use resource_mesh_portal_serde::version::latest::operation::{ExtOperation, Operation};
use resource_mesh_portal_serde::version::latest::portal::{inlet, outlet};
use resource_mesh_portal_serde::version::latest::resource::Status;

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

#[derive(Debug)]
pub struct Exchange {
    pub id: ExchangeId,
    pub tx: tokio::sync::oneshot::Sender<inlet::Response>,
}

#[derive(Debug)]
enum PortalCall {
    FrameIn(inlet::Frame),
    FrameOut(outlet::Frame),
    Exchange(Exchange)
}

pub struct Portal {
    pub info: Info,
    outlet_tx: mpsc::Sender<outlet::Frame>,
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

    pub fn new(info: Info, outlet_tx: mpsc::Sender<outlet::Frame>, inlet_rx: mpsc::Receiver<inlet::Frame>, logger: fn(log:Log) ) -> Self {

        let (mux_tx,mux_rx) = tokio::sync::mpsc::channel(1024);
        let (status_tx,status_rx) = tokio::sync::broadcast::channel(8);
        let (call_tx,mut call_rx) = tokio::sync::mpsc::channel(1024);
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
            let mut exchanges:HashMap<ExchangeId,oneshot::Sender<inlet::Response>> =  HashMap::new();
            let mux_tx= mux_tx.clone();
            let outlet_tx = outlet_tx.clone();
            let info = info.clone();
            let status_tx = status_tx.clone();
            tokio::spawn(async move {

                match outlet_tx.send( outlet::Frame::Init(info.clone())).await {
                    Result::Ok(_) => {}
                    Result::Err(err) => {
                        logger(Log::Fatal("FATAL: could not send Frame::Init".to_string()));
                        mux_tx.try_send(MuxCall::Remove(Identifier::Key(info.key.clone()))).unwrap_or_default();
                        return;
                    }
                }
                while let Option::Some(command) = call_rx.recv().await {
                    match command {
                        PortalCall::FrameIn(frame) => {
                            match frame {
                                inlet::Frame::Log(log) => {
                                    (logger)(log);
                                }
                                inlet::Frame::Command(_) => {}
                                inlet::Frame::Request(request) => {
                                    match &request.kind {
                                        ExchangeKind::None=> {
                                            logger(Log::Fatal("FATAL: received request with an invalid 'ExchangeKind::None'".to_string()))
                                        }
                                        ExchangeKind::Notification => {
                                            for to in &request.to {
                                                let request = request_message::inlet::Request::from( request.clone(), Identifier::Key(info.key.clone()), to.clone() );
                                                let result = mux_tx.send_timeout(MuxCall::MessageIn(message::inlet::Message::Request(request)), Duration::from_secs(info.config.frame_timeout.clone())).await;
                                                if let Result::Err(_err) = result {
                                                    logger(Log::Fatal("FATAL: send timeout error request_tx".to_string()))
                                                }
                                            }
                                        }
                                        ExchangeKind::RequestResponse(exchange_id) => {
                                            if request.to.len() != 1 {
                                                let response = outlet::Response{
                                                    from: Identifier::Key(info.key.clone()),
                                                    exchange_id: exchange_id.clone(),
                                                    signal: ResponseEntity::Error("a RequestResponse message must have one and only one to recipient.".to_string())
                                                };
                                                let result = outlet_tx.send_timeout(outlet::Frame::Response(response), Duration::from_secs(info.config.frame_timeout.clone()) ).await;
                                                if let Result::Err(_err) = result {
                                                    logger(Log::Fatal("FATAL: frame timeout error exit_tx".to_string()));
                                                }
                                            } else {
                                                let to = request.to.first().expect("expected to identifier").clone();
                                                let request = request_message::inlet::Request::from( request.clone(), Identifier::Key(info.key.clone()), to );
                                                let result = mux_tx.send_timeout(MuxCall::MessageIn(message::inlet::Message::Request(request)), Duration::from_secs(info.config.frame_timeout.clone())).await;
                                                if let Result::Err(_err) = result {
                                                    logger(Log::Fatal("FATAL: frame timeout error request_tx".to_string()));
                                                }
                                            }
                                        }
                                    }

                                }
                                inlet::Frame::Response(response) => {
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
                                inlet::Frame::BinParcel(_) => {}
                                inlet::Frame::Status(status) => {
                                    status_tx.send(status).unwrap_or_default();
                                }
                                inlet::Frame::Close(_) => {}
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

    pub async fn send(&self, frame: outlet::Frame ) -> Result<(), Error> {
        self.outlet_tx.send_timeout(frame, Duration::from_secs( self.info.config.frame_timeout.clone() ) ).await?;
        Ok(())
    }

    pub async fn exchange(&self, request: outlet::Request ) -> Result<inlet::Response, Error> {
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
        self.outlet_tx.try_send(outlet::Frame::Close(CloseReason::Done)).unwrap_or(());
    }

    pub async fn init(&mut self) -> Result<(), Error> {
        if self.status != PortalStatus::None {
            let message = format!("{} has already received the init signal.",self.info.kind.to_string());
            return Err(anyhow!(message));
        }

        self.status = PortalStatus::Initializing;

        self.outlet_tx.try_send(outlet::Frame::Init(self.info.clone()) )?;
        let mut status_rx = self.status_tx.subscribe();
        let (tx,rx) = tokio::sync::oneshot::channel();
        let config = self.info.config.clone();
        let kind = self.info.kind.clone();
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
    Select{ selector: fn(info:&Info)->bool, tx: oneshot::Sender<Vec<Info>> },
    MessageIn(inlet::Message),
    MessageOut(outlet::Message)
}

pub mod message {

    pub mod inlet {
        use resource_mesh_portal_serde::version::latest::operation::Operation;
        use crate::message::generic;

        pub type Message = generic::Message<Operation>;
    }

    pub mod outlet {
        use resource_mesh_portal_serde::version::latest::operation::ExtOperation;
        use crate::message::generic;

        pub type Message = generic::Message<ExtOperation>;
    }

    pub mod generic {
        use resource_mesh_portal_serde::message::generic::{Request, Response};
        use resource_mesh_portal_serde::version::latest::id::Identifier;

        pub enum Message<OPERATION> {
            Request(Request<OPERATION>),
            Response(Response)
        }

        impl<OPERATION> Message<OPERATION> {
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
    }

}





pub trait Router: Send+Sync {
    fn route( &self, message: inlet::Message );
    fn logger( &self, message: &str ) {
        println!("{}", message );
    }
}

pub struct PortalMuxer {
    portals: HashMap<Identifier,Portal>,
    router: Box<dyn Router>,
    address_to_key: HashMap<Address,Key>,
    key_to_address: HashMap<Key,Address>,
    mux_tx: mpsc::Sender<MuxCall>,
    mux_rx: mpsc::Receiver<MuxCall>,
}

impl PortalMuxer {
    pub fn new( mux_tx: mpsc::Sender<MuxCall>, mux_rx: mpsc::Receiver<MuxCall>, router: Box<dyn Router> ) {

        let mut muxer = Self {
            portals: HashMap::new(),
            address_to_key: HashMap::new(),
            key_to_address: HashMap::new(),
            router,
            mux_tx,
            mux_rx
        };

        tokio::spawn( async move {

            let mut ids = vec![];
            let mut futures = vec![];

            for (key,portal) in &mut muxer.portals {
                futures.push( portal.mux_rx.recv().boxed() );
                ids.push(key.clone());
            }

            futures.push( muxer.mux_rx.recv().boxed() );

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
                            let kind = portal.info.kind.clone();
                            let address = portal.info.address.clone();
                            muxer.key_to_address.insert(portal.info.key.clone(), portal.info.address.clone() );
                            muxer.address_to_key.insert(portal.info.address.clone(), portal.info.key.clone() );
                            muxer.portals.insert(Identifier::Key(portal.info.key.clone()), portal );
                            muxer.router.logger(format!("INFO: {} add to portal muxer at address {}", kind.to_string(), address ).as_str() );
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

                                    muxer.router.logger(format!("INFO: {} removed from portal muxer at address {}", portal.info.kind.to_string(), portal.info.address ).as_str() );
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
                                        outlet::Message::Request(request) => {
                                            portal.call_tx.try_send( PortalCall::FrameOut( outlet::Frame::Request(request.into())));
                                        }
                                        outlet::Message::Response(response) => {
                                            portal.call_tx.try_send( PortalCall::FrameOut( outlet::Frame::Response(response.into())));
                                        }
                                    }
                                }
                                None => {}
                            }
                        }
                        MuxCall::Select { selector, tx } => {
                            let mut rtn = vec![];
                            for portal in muxer.portals.values() {
                                if selector(&portal.info) {
                                    rtn.push(portal.info.clone());
                                }
                            }
                            tx.send(rtn).unwrap_or_default();
                        }
                    }
                }
            }

        } );
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