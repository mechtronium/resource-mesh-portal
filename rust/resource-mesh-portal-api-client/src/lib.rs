#[macro_use]
extern crate async_trait;

#[macro_use]
extern crate anyhow;

use resource_mesh_portal_serde::config::{Config, Info};
use resource_mesh_portal_serde::mesh::outlet::Frame;
use resource_mesh_portal_serde::{
    mesh, Entity, ExchangeId, ExchangeKind, Identifier, Log, Port, Signal, Status,
};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use uuid::Uuid;
use dashmap::DashMap;
use anyhow::Error;
use std::cell::Cell;

#[async_trait]
pub trait PortalCtrl : Sync+Send {
    async fn init(&mut self) -> Result<(), Error>;
    async fn request(&self, request: Request ) -> Result<(), Error>;
}

pub fn log(message: &str) {
    println!("{}",message);
}

pub trait Inlet: Sync+Send {
    fn send(&self, frame: mesh::inlet::Frame);
}

pub trait Outlet: Sync+Send {
    fn receive(&mut self, frame: mesh::outlet::Frame);
}

pub struct StatusChamber {
    pub status: Status
}

impl StatusChamber{
    pub fn new( status: Status ) -> Self {
        Self {
            status
        }
    }
}

pub struct PortalSkel {
    pub info: Info,
    pub inlet: Box<dyn Inlet>,
    pub logger: fn(message: &str),
    pub exchanges: DashMap<ExchangeId, oneshot::Sender<mesh::outlet::Response>>,
    pub status: RwLock<StatusChamber>,
}

impl PortalSkel {
    pub fn status(&self) -> Status {
        (*self.status.read().expect("expected status read lock")).status.clone()
    }

    pub fn set_status(&mut self, status: Status) {
        {
            self.status.write().expect("expected to get status write lock").status = status.clone();
        }
        self.inlet.send(mesh::inlet::Frame::Status(status));
    }

}


pub struct Portal {
    pub skel: Arc<PortalSkel>,
    pub ctrl: Arc<dyn PortalCtrl>,
    pub inlet_api: Arc<InletApi>
}

impl Portal {
    pub async fn new(
        info: Info,
        inlet: Box<dyn Inlet>,
        ctrl_factory: fn(skel: Arc<PortalSkel>, portal: InletApi) -> Box<dyn PortalCtrl>,
        logger: fn(message: &str),
    ) -> Result<Arc<Portal>, Error> {

        let status = RwLock::new(StatusChamber::new( Status::Initializing ));
        inlet.send(mesh::inlet::Frame::Status(Status::Initializing));
        let exchanges = DashMap::new();
        let skel = Arc::new( PortalSkel {
            info: info.clone(),
            inlet,
            logger,
            exchanges,
            status
        });

        let inlet_api = InletApi::new(skel.clone() );
        let mut ctrl = ctrl_factory(skel.clone(), inlet_api);
        ctrl.init();
        let ctrl = ctrl.into();
        let inlet_api = Arc::new(InletApi::new(skel.clone() ));
        let mut portal = Self {
            skel: skel.clone(),
            ctrl,
            inlet_api
        };

        Ok(Arc::new(portal))
    }

    pub fn log( &self, log: Log ) {
        self.skel.inlet.send(mesh::inlet::Frame::Log(log));
    }

}

#[async_trait]
impl Outlet for Portal {
    fn receive(&mut self, frame: Frame) {
        match self.skel.status() {
            Status::Ready => match frame {
                Frame::CommandEvent(_) => {}
                Frame::Request(request) => {
                    let ctrl = self.ctrl.clone();
                    let inlet_api = self.inlet_api.clone();
                    let skel = self.skel.clone();
                    tokio::spawn( async move {
                        let (tx, rx) = oneshot::channel();
                        let request = Request::from(request, tx, skel.logger );
                        match ctrl.request(request).await {
                            Ok(_) => {
                                tokio::spawn(async move {
                                    let result = rx.await;
                                    if let Result::Ok(response) = result {
                                        inlet_api.respond( response );
                                    }
                                });
                            }
                            Err(error) => {
                                inlet_api.log(Log::Error(error.to_string()));
                            }
                        }
                    });
                }
                Frame::Response(response) => {
                    if let Option::Some((_,tx)) =
                        self.skel.exchanges.remove(&response.exchange_id)
                    {
                        tx.send(response);
                    } else {
                        (self.skel.logger)("SEVERE: do not have a matching exchange_id for response");
                    }
                }
                Frame::BinParcel(_) => {}
                Frame::Shutdown => {}
                _ => {
                    (self.skel.logger)(format!("SEVERE: frame ignored because status: '{}' does not allow handling of frame '{}'",self.skel.status().to_string(),frame.to_string()).as_str());
                }
            },
            _ => {
                (self.skel.logger)(format!("SEVERE: frame ignored because status: '{}' does not allow handling of frame '{}'",self.skel.status().to_string(),frame.to_string()).as_str());
            }
        }
    }
}

pub struct InletApi {
    skel: Arc<PortalSkel>,
}

impl InletApi {
    pub fn new(skel: Arc<PortalSkel> ) -> Self {
        Self {
            skel
        }
    }

    pub fn log( &self, log: Log ) {
        self.skel.inlet.send(mesh::inlet::Frame::Log(log));
    }

    pub fn info(&self) -> Info {
        self.skel.info.clone()
    }



    pub fn notify(&self, request: mesh::inlet::Request) {
        let mut request = request;
        if let ExchangeKind::None = request.kind {
        } else {
            self.log(Log::Warn("ExchangeKind is replaced in 'notify' or 'exchange' method and should be preset to ExchangeKind::None".to_string()));
        }
        request.kind = ExchangeKind::Notification;
        self.skel.inlet.send(mesh::inlet::Frame::Request(request));
    }

    pub async fn exchange(
        &mut self,
        request: mesh::inlet::Request
    ) -> Result<mesh::outlet::Response, Error> {
        if let ExchangeKind::None = request.kind {
        } else {
            self.skel.inlet.send(mesh::inlet::Frame::Log(Log::Warn("ExchangeKind is replaced in 'notify' or 'exchange' method and should be preset to ExchangeKind::None".to_string())));
        }
        let mut request = request;
        let exchangeId: ExchangeId = Uuid::new_v4().to_string();
        request.kind = ExchangeKind::RequestResponse(exchangeId.clone());
        let (tx,rx) = oneshot::channel();
        self.skel.exchanges.insert(exchangeId, tx);
        self.skel.inlet.send(mesh::inlet::Frame::Request(request));

        let result = tokio::time::timeout(Duration::from_secs(self.skel.info.config.response_timeout.clone()),rx).await;
        Ok(result??)
    }

    pub fn respond( &self, response: mesh::inlet::Response ) {
        self.skel.inlet.send( mesh::inlet::Frame::Response(response) );
    }
}

pub struct Request {
    pub from: Identifier,
    pub port: Port,
    pub entity: Entity,
    pub kind: ExchangeKind,
    tx: oneshot::Sender<mesh::inlet::Response>,
    logger: fn(message: &str ),
}

impl Request {

    pub fn from( request: mesh::outlet::Request, tx: oneshot::Sender<mesh::inlet::Response>, logger: fn(message: &str ) ) -> Self {
        Self {
            from: request.from,
            port: request.port,
            entity: request.entity,
            kind: request.kind,
            tx,
            logger,
        }
    }

    pub fn ok(self) {
        if self.can_reply() {
            self.respond(Signal::Ok(Entity::Empty));
        }
    }

    pub fn error(self, message: String) {
        if self.can_reply() {
            self.respond(Signal::Error(message));
        } else {
            (self.logger)(
                format!(
                    "SEVERE: cannot respond with error to request notification message: '{}'",
                    message
                ).as_str()
            );
        }
    }

    pub fn can_reply(&self) -> bool {
        match self.kind {
            ExchangeKind::RequestResponse(_) => true,
            _ => false,
        }
    }

    pub fn respond(self, signal: Signal) -> Result<(), Error> {
        if let ExchangeKind::RequestResponse(exchange) = &self.kind {
            let response = mesh::inlet::Response {
                to: self.from.clone(),
                exchange_id: exchange.clone(),
                signal,
            };
            self.tx.send(response);
            Ok(())
        } else {
            Err(
                anyhow!("a request can only be responded to if it is kind == ExchangeKind::RequestResponse")
            )
        }
    }
}

pub mod example {
    use crate::{PortalCtrl, InletApi, Request, PortalSkel};
    use resource_mesh_portal_serde::config::Info;
    use resource_mesh_portal_serde::{mesh, Entity, ExtOperation, Operation, PortRequest, Signal, Payload};
    use std::sync::Arc;
    use anyhow::Error;

    pub struct HelloCtrl {
        pub skel: Arc<PortalSkel>,
        pub inlet_api: InletApi
    }

    impl HelloCtrl {
        fn new(skel: Arc<PortalSkel>, inlet_api: InletApi) -> Box<Self> {
            Box::new(Self { skel, inlet_api })
        }
    }

    #[async_trait]
    impl PortalCtrl for HelloCtrl {
        async fn init(&mut self) -> Result<(), Error> {
            let mut request =
                mesh::inlet::Request::new(Operation::Ext(ExtOperation::Port(PortRequest {
                    port: "hello-world".to_string(),
                    entity: Entity::Empty,
                })));

            // send this request to itself
            request.to.push( self.inlet_api.skel.info.key.clone() );

            let response = self.inlet_api.exchange(request).await?;

            if let Signal::Ok(Entity::Payload(Payload::Text(text))) = response.signal {
                println!("{}",text);
            } else {
                return Err(anyhow!("unexpected signal"));
            }

            Ok(())
        }

        async fn request(&self, request: Request) -> Result<(), Error> {
            match request.port.as_str() {
                "hello-world" => {
                    request.respond(Signal::Ok(Entity::Payload(Payload::Text("Hello World!".to_string()))) );
                }
                _ => {
                    request.error("unrecognized port".to_string() );
                }
            }
            Ok(())
        }
    }
}
