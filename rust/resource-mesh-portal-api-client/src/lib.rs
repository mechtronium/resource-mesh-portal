#[macro_use]
extern crate async_trait;

pub mod error;

use crate::error::Error;
use resource_mesh_portal_serde::config::{Config, Info};
use resource_mesh_portal_serde::mesh::outlet::Frame;
use resource_mesh_portal_serde::{
    mesh, Entity, ExchangeId, ExchangeKind, Identifier, Log, Port, Signal, Status,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use uuid::Uuid;

#[async_trait]
pub trait PortalCtrl {
    async fn init(&mut self, info: Info ) -> Result<(),Error>;
    async fn request(&self, request: Request ) -> Result<(),Error>;
}

pub fn log(message: &str) {
    println!(message);
}

pub trait Inlet {
    fn send(&self, frame: mesh::inlet::Frame);
}

pub trait Outlet {
    fn receive(&mut self, frame: mesh::outlet::Frame);
}

pub struct Portal {
    pub info: Info,
    pub status: Status,
    inlet: Arc<dyn Inlet>,
    portal_inlet: Arc<PortalInlet>,
    logger: fn(message: &str),
    ctrl: Box<dyn PortalCtrl>,
}

impl Portal {
    pub async fn new(
        info: Info,
        inlet: Arc<dyn Inlet>,
        ctrl_factory: fn(portal: Arc<PortalInlet>) -> Box<dyn PortalCtrl>,
        logger: fn(message: &str),
    ) -> Result<Arc<Portal>, Error> {
        let portal_inlet = Arc::new(PortalInlet::new(info.clone(), inlet.clone()));
        inlet.send(mesh::inlet::Frame::Status(Status::Initializing));
        let ctrl = ctrl_factory(portal_inlet.clone());
        let mut portal = Arc::new(Self {
            info: info.clone(),
            status: Status::Initializing,
            inlet,
            portal_inlet,
            logger,
            ctrl,
        });

        portal.init().await?;

        Ok(portal)
    }

    pub async fn init(&mut self) -> Result<(),Error> {
        self.ctrl.init( self.info.clone() ).await
    }

}

impl Outlet for Portal {
    fn receive(&mut self, frame: Frame) {
        match self.status {
            Status::Ready => match frame {
                Frame::CommandEvent(_) => {}
                Frame::Request(_) => {}
                Frame::Response(response) => {
                    if let Option::Some(tx) =
                        self.portal_inlet.exchanges.remove(&response.exchange_id)
                    {
                        tx.send(response);
                    } else {
                        self.logger("SEVERE: do not have a matching exchange_id for response");
                    }
                }
                Frame::BinParcel(_) => {}
                Frame::Shutdown => {}
                _ => {
                    self.logger(format!("SEVERE: frame ignored because status: '{}' does not allow handling of frame '{}'",self.status.to_string(),frame.to_string()).as_str());
                }
            },
            _ => {
                self.logger(format!("SEVERE: frame ignored because status: '{}' does not allow handling of frame '{}'",self.status.to_string(),frame.to_string()).as_str());
            }
        }
    }
}

pub struct PortalInlet {
    info: Info,
    status: Status,
    inlet: Arc<dyn Inlet>,
    exchanges: HashMap<ExchcangeId, oneshot::Sender<mesh::outlet::Response>>,
}

impl PortalInlet {
    pub fn new(info: Info, inlet: Arc<dyn Inlet>) -> Self {
        Self {
            info,
            exchanges: HashMap::new(),
            status: Status::Unknown,
            inlet,
        }
    }

    pub fn info(&self) -> Info {
        self.info.clone()
    }

    pub fn status(&self) -> Status {
        self.status.clone()
    }

    pub fn set_status(&mut self, status: Status) {
        self.status = status.clone();
        self.inlet.send(mesh::inlet::Frame::Status(status));
    }

    pub fn notify(&self, request: mesh::inlet::Request) {
        let mut request = request;
        if let ExchangeKind::None = request.kind {
        } else {
            self.inlet.send(mesh::inlet::Frame(Log::Warn("ExchangeKind is replaced in 'notify' or 'exchange' method and should be preset to ExchangeKind::None".to_string())));
        }
        request.kind = ExchangeKind::Notification;
        self.inlet.send(mesh::inlet::Frame::Request(request));
    }

    pub async fn exchange(
        &mut self,
        request: mesh::inlet::Request
    ) -> Result<mesh::outlet::Response,Error> {
        if let ExchangeKind::None = request.kind {
        } else {
            self.inlet.send(mesh::inlet::Frame(Log::Warn("ExchangeKind is replaced in 'notify' or 'exchange' method and should be preset to ExchangeKind::None".to_string())));
        }
        let mut request = request;
        let exchangeId: ExchangeId = Uuid::new_v4().to_string();
        request.kind = ExchangeKind::RequestResponse(exchangeId.clone());
        let (tx,rx) = oneshot::channel();
        self.exchanges.insert(exchangeId, tx);
        self.inlet.send(mesh::inlet::Frame::Request(request));

        let result = tokio::time::timeout(Duration::from_secs(self.info.config.response_timeout.clone()),rx).await;
        Ok(result??)
    }
}

pub struct Request {
    pub from: Identifier,
    pub port: Port,
    pub entity: Entity,
    pub kind: ExchangeKind,
    callback: fn(response: mesh::inlet::Response),
    logger: fn(message: String),
}

impl Request {
    pub fn ok(self) {
        if self.can_reply() {
            self.respond(Signal::Ok(Entity::Empty));
        }
    }

    pub fn error(self, message: String) {
        if self.can_reply() {
            self.respond(Signal::Error(message));
        } else {
            logger(
                format!(
                    "SEVERE: cannot respond with error to request notification message: '{}'",
                    message
                )
                .as_str(),
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
            self.callback(response);
            Ok(())
        } else {
            Err(
                "a request can only be responded to if it is kind == ExchangeKind::RequestResponse"
                    .into(),
            )
        }
    }
}

pub mod example {
    use crate::error::Error;
    use crate::{PortalCtrl, PortalInlet, Request};
    use resource_mesh_portal_serde::config::Info;
    use resource_mesh_portal_serde::{mesh, Entity, ExtOperation, Operation, PortRequest, Signal, Payload};
    use std::sync::Arc;

    pub struct HelloCtrl {
        pub portal_inlet: Arc<PortalInlet>,
    }

    impl HelloCtrl {
        fn new(portal_inlet: Arc<PortalInlet>) -> Box<Self> {
            Box::new(Self { portal_inlet })
        }
    }

    impl PortalCtrl for HelloCtrl {
        async fn init(&mut self, info: Info) -> Result<(), Error> {
            let mut request =
                mesh::inlet::Request::new(Operation::Ext(ExtOperation::Port(PortRequest {
                    port: "hello-world".to_string(),
                    entity: Entity::Empty,
                })));

            // send this request to itself
            request.to.push( info.key.clone() );

            let response = self.portal_inlet.exchange(request).await?;

            if let Signal::Ok(Entity::Payload(Payload::Text(text))) = response.signal {
                println!(text);
            } else {
                return Err("unexpected signal".into());
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
