#[macro_use]
extern crate async_trait;

#[macro_use]
extern crate anyhow;



use std::sync::{Arc, RwLock};
use std::time::Duration;

use anyhow::Error;
use dashmap::DashMap;
use tokio::sync::oneshot;
use uuid::Uuid;


use std::prelude::rust_2021::TryFrom;
use std::ops::Deref;
use std::collections::HashMap;
use tokio::sync::watch::Receiver;
use client::{Request,RequestContext};
use resource_mesh_portal_serde::std_logger;
use resource_mesh_portal_serde::version::latest::http::{HttpRequest, HttpResponse};
use resource_mesh_portal_serde::version::latest::portal::{inlet, outlet};
use resource_mesh_portal_serde::version::latest::resource::Status;
use resource_mesh_portal_serde::version::latest::messaging::{ExchangeId, ExchangeKind};
use resource_mesh_portal_serde::version::latest::config::Info;
use resource_mesh_portal_serde::version::latest::log::Log;
use resource_mesh_portal_serde::version::latest::operation::{ExtOperation, PortOperation};
use resource_mesh_portal_serde::version::latest::delivery::Entity;
use resource_mesh_portal_serde::version::latest::delivery::ResponseEntity;


struct EmptySkel {

}

#[async_trait]
pub trait PortalCtrl: Sync+Send {
    async fn init(&mut self) -> Result<(), Error>
    {
        Ok(())
    }

    fn ports(&self) -> HashMap<String,Box<dyn PortCtrl>> {
        HashMap::new()
    }

    async fn http_request( &self, request: Request<HttpRequest> ) -> Result<HttpResponse,Error> {
        let response = HttpResponse {
            headers: Default::default(),
            code: 404,
            body: None
        };
        Ok(response)
    }
}


#[async_trait]
pub trait PortCtrl: Sync+Send {
    async fn request( &self, request: Request<PortOperation> ) -> Result<Option<ResponseEntity>,Error>{
        Ok(Option::None)
    }
}


pub fn log(message: &str) {
    println!("{}",message);
}

pub trait Inlet: Sync+Send {
    fn send_frame(&self, frame: inlet::Frame);
}

pub trait Outlet: Sync+Send {
    fn receive(&mut self, frame: outlet::Frame);
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

pub type Exchanges = Arc<DashMap<ExchangeId, oneshot::Sender<outlet::Response>>>;
pub type PortalStatus = Arc<RwLock<StatusChamber>>;


#[derive(Clone)]
pub struct PortalSkel {
    pub info: Info,
    pub inlet: Arc<dyn Inlet>,
    pub logger: fn(message: &str),
    pub exchanges: Exchanges,
    pub status: PortalStatus
}

impl PortalSkel {
    pub fn status(&self) -> Status {
        (*self.status.read().expect("expected status read lock")).status.clone()
    }

    pub fn set_status(&mut self, status: Status) {
        {
            self.status.write().expect("expected to get status write lock").status = status.clone();
        }
        self.inlet.send_frame(inlet::Frame::Status(status));
    }

    pub fn api(&self) -> InletApi {
        InletApi::new( self.info.clone(), self.inlet.clone(), self.exchanges.clone(), std_logger )
    }

}


pub struct Portal {
    pub skel: PortalSkel,
    pub ctrl: Arc<dyn PortalCtrl>,
    pub ports: Arc<HashMap<String,Box<dyn PortCtrl>>>,
}

impl Portal {
    pub async fn new(
        info: Info,
        inlet: Box<dyn Inlet>,
        ctrl_factory: fn(skel: PortalSkel) -> Box<dyn PortalCtrl>,
        logger: fn(message: &str)
    ) -> Result<Arc<Portal>, Error> {

        let inlet :Arc<dyn Inlet>= inlet.into();
        let status = Arc::new(RwLock::new(StatusChamber::new( Status::Initializing )));
        inlet.send_frame(inlet::Frame::Status(Status::Initializing));
        let exchanges = Arc::new(DashMap::new());
        let skel =  PortalSkel {
            info: info.clone(),
            inlet,
            logger,
            exchanges,
            status
        };

        let mut ctrl = ctrl_factory(skel.clone());
        let ports = Arc::new(ctrl.ports());
        ctrl.init().await?;
        let ctrl = ctrl.into();
        let portal = Self {
            skel: skel.clone(),
            ctrl,
            ports
        };

        Ok(Arc::new(portal))
    }

    pub fn log( &self, log: Log ) {
        self.skel.inlet.send_frame(inlet::Frame::Log(log));
    }

}

#[async_trait]
impl Outlet for Portal {
    fn receive(&mut self, frame: outlet::Frame) {
        match self.skel.status() {
            Status::Ready => match frame {
                outlet::Frame::CommandEvent(_) => {}
                outlet::Frame::Request(request) => {
                    let ctrl = self.ctrl.clone();
                    let inlet_api = self.skel.api();
                    let skel = self.skel.clone();
                    let context = RequestContext::new(skel.info.clone(), skel.logger );
                    let ports = self.ports.clone();
                    let from = request.from.clone();
                    let kind = request.kind.clone();
                    tokio::spawn( async move {
                        match request.operation.clone() {
                            ExtOperation::Http(_) => {
                                if let ExchangeKind::RequestResponse(exchange_id) = &kind
                                {
                                    let result = Request::try_from_http(request, context);
                                    match result {
                                        Ok(request) => {
                                            let path = request.path.clone();
                                            let result = ctrl.http_request(request).await;
                                            match result {
                                                Ok(response) => {
                                                    let response = inlet::Response {
                                                        to: from,
                                                        exchange_id:exchange_id.clone(),
                                                        signal: ResponseEntity::Ok(Entity::HttpResponse(response))
                                                    };
                                                    inlet_api.respond( response );
                                                }
                                                Err(err) => {
                                                    (skel.logger)(format!("ERROR: HttpRequest.path: '{}' error: '{}' ",  path, err.to_string()).as_str());
                                                    let response = inlet::Response {
                                                        to: from,
                                                        exchange_id:exchange_id.clone(),
                                                        signal: ResponseEntity::Ok(Entity::HttpResponse(HttpResponse::server_side_error()))
                                                    };
                                                    inlet_api.respond( response );
                                                }
                                            }
                                        }
                                        Err(err) => {
                                            (skel.logger)(format!("FATAL: could not modify HttpRequest into Request<HttpRequest>: {}", err.to_string()).as_str());
                                        }
                                    }
                                } else {
                                    (skel.logger)("FATAL: http request MUST be of ExchangeKind::RequestResponse");
                                }
                            }
                            ExtOperation::Port(port_request) => {
                                match ports.get(&port_request.port ) {
                                    Some(port) => {
                                        let result = Request::try_from_port(request, context );
                                        match result {
                                            Ok(request) => {
                                                let request_from = request.from.clone();
                                                let result = port.request(request).await;
                                                match result {
                                                    Ok(response) => {
                                                        match response {
                                                            Some(signal) => {
                                                                if let ExchangeKind::RequestResponse(exchange_id) = &kind
                                                                {
                                                                   let response = inlet::Response {
                                                                       to: request_from,
                                                                       exchange_id: exchange_id.clone(),
                                                                       signal
                                                                   };

                                                                   inlet_api.respond(response);
                                                                } else {
                                                                    let message = format!("WARN: PortOperation.port '{}' generated a response to a ExchangeKind::Notification", port_request.port);
                                                                    (skel.logger)(message.as_str());
                                                                }
                                                            }
                                                            None => {
                                                                let message = format!("ERROR: PortOperation.port '{}' generated no response", port_request.port);
                                                                (skel.logger)(message.as_str());
                                                                if let ExchangeKind::RequestResponse(exchange_id) = &kind
                                                                {
                                                                    let response = inlet::Response {
                                                                        to: request_from,
                                                                        exchange_id: exchange_id.clone(),
                                                                        signal: ResponseEntity::Error(message)
                                                                    };
                                                                    inlet_api.respond(response);
                                                                }
                                                            }
                                                        }
                                                    }
                                                    Err(err) => {
                                                        let message = format!("ERROR: PortOperation.port '{}' message: '{}'", port_request.port, err.to_string());
                                                        (skel.logger)(message.as_str());
                                                        if let ExchangeKind::RequestResponse(exchange_id) = &kind
                                                        {
                                                            let response = inlet::Response {
                                                                to: request_from,
                                                                exchange_id: exchange_id.clone(),
                                                                signal: ResponseEntity::Error(message)
                                                            };
                                                            inlet_api.respond(response);
                                                        }
                                                    }
                                                }

                                            }
                                            Err(err) => {
                                                let message = format!("FATAL: could not modify PortOperation into Request<PortOperation>: {}", err.to_string());
                                                (skel.logger)(message.as_str());
                                                if let ExchangeKind::RequestResponse(exchange_id) = &kind
                                                {
                                                    let response = inlet::Response {
                                                        to: from,
                                                        exchange_id: exchange_id.clone(),
                                                        signal: ResponseEntity::Error(message)
                                                    };
                                                    inlet_api.respond(response);
                                                }
                                            }
                                        }

                                    }
                                    None => {
                                        let message =format!("ERROR: message port: '{}' not defined ", port_request.port );
                                        (skel.logger)(message.as_str());
                                        if let ExchangeKind::RequestResponse(exchange_id) = &kind
                                        {
                                            let response = inlet::Response {
                                                to: from,
                                                exchange_id: exchange_id.clone(),
                                                signal: ResponseEntity::Error(message)
                                            };
                                            inlet_api.respond(response);
                                        }
                                    }
                                }
                            }
                        }
                    });
                }
                outlet::Frame::Response(response) => {
                    if let Option::Some((_,tx)) =
                        self.skel.exchanges.remove(&response.exchange_id)
                    {
                        tx.send(response).unwrap_or(());
                    } else {
                        (self.skel.logger)("SEVERE: do not have a matching exchange_id for response");
                    }
                }
                outlet::Frame::BinParcel(_) => {}
                outlet::Frame::Close(_) => {}
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
    info: Info,
    inlet: Arc<dyn Inlet>,
    exchanges: Exchanges,
    logger: fn( log: Log )
}

impl InletApi {
    pub fn new(info: Info, inlet: Arc<dyn Inlet>, exchanges: Exchanges, logger: fn( log: Log ) ) -> Self {
        Self {
            info,
            inlet,
            exchanges,
            logger
        }
    }


    pub fn notify(&self, request: inlet::Request) {
        let mut request = request;
        if let ExchangeKind::None = request.kind {
        } else {
            (self.logger)(Log::Warn("ExchangeKind is replaced in 'notify' or 'exchange' method and should be preset to ExchangeKind::None".to_string()));
        }
        request.kind = ExchangeKind::Notification;
        self.inlet.send_frame(inlet::Frame::Request(request));
    }

    pub async fn exchange(
        &mut self,
        request: inlet::Request
    ) -> Result<outlet::Response, Error> {
        if let ExchangeKind::None = request.kind {
        } else {
            self.inlet.send_frame(inlet::Frame::Log(Log::Warn("ExchangeKind is replaced in 'notify' or 'exchange' method and should be preset to ExchangeKind::None".to_string())));
        }
        let mut request = request;
        let exchange_id: ExchangeId = Uuid::new_v4().to_string();
        request.kind = ExchangeKind::RequestResponse(exchange_id.clone());
        let (tx,rx) = oneshot::channel();
        self.exchanges.insert(exchange_id, tx);
        self.inlet.send_frame(inlet::Frame::Request(request));

        let result = tokio::time::timeout(Duration::from_secs(self.info.config.response_timeout.clone()),rx).await;
        Ok(result??)
    }

    pub fn respond( &self, response: inlet::Response ) {
        self.inlet.send_frame( inlet::Frame::Response(response) );
    }
}

pub mod client {
    use std::ops::Deref;
    use anyhow::Error;
    use resource_mesh_portal_serde::version::latest::portal::outlet;
    use resource_mesh_portal_serde::version::latest::id::Identifier;
    use resource_mesh_portal_serde::version::latest::operation::{ExtOperation, PortOperation};
    use resource_mesh_portal_serde::version::latest::config::Info;
    use resource_mesh_portal_serde::version::latest::http::HttpRequest;
    use resource_mesh_portal_serde::version::latest::delivery::ResponseEntity;

    #[derive(Clone)]
    pub struct RequestContext {
        pub portal_info: Info,
        pub logger: fn(message: &str),
    }

    impl RequestContext {
        pub fn new(portal_info: Info, logger: fn(message: &str)) -> Self {
            Self {
                portal_info,
                logger
            }
        }
    }

    pub struct Request<REQUEST> {
        pub context: RequestContext,
        pub from: Identifier,
        pub request: REQUEST
    }

    impl<REQUEST> Deref for Request<REQUEST> {
        type Target = REQUEST;

        fn deref(&self) -> &Self::Target {
            &self.request
        }
    }

    impl Request<HttpRequest> {
        pub fn try_from_http(request: outlet::Request, context: RequestContext) -> Result<Request<HttpRequest>, Error> {
            if let ExtOperation::Http(http_request) = request.operation {
                Ok(Self {
                    context,
                    from: request.from,
                    request: http_request,
                })
            } else {
                Err(anyhow!("can only create Request<PortOperation> from ExtOperation::Port"))
            }
        }
    }

    impl Request<PortOperation> {
        pub fn try_from_port(request: outlet::Request, context: RequestContext) -> Result<Request<PortOperation>, Error> {
            if let ExtOperation::Port(port_request) = request.operation {
                Ok(Self {
                    context,
                    from: request.from,
                    request: port_request,
                })
            } else {
                Err(anyhow!("can only create Request<PortOperation> from ExtOperation::Port"))
            }
        }
    }
}


pub mod example {
    use std::sync::Arc;

    use anyhow::Error;


    use crate::{InletApi, PortalCtrl, PortalSkel, Request, inlet};
    use std::collections::HashMap;
    use resource_mesh_portal_serde::version::latest::operation::{Operation, ExtOperation, PortOperation};
    use resource_mesh_portal_serde::version::latest::delivery::{Entity, ResponseEntity,Payload};
    use resource_mesh_portal_serde::version::latest::id::Identifier;

    pub struct HelloCtrl {
        pub skel: Arc<PortalSkel>,
        pub inlet_api: InletApi
    }

    impl HelloCtrl {
        #[allow(dead_code)]
        fn new(skel: Arc<PortalSkel>, inlet_api: InletApi) -> Box<Self> {
            Box::new(Self { skel, inlet_api } )
        }
    }

    #[async_trait]
    impl PortalCtrl for HelloCtrl {

        async fn init(&mut self) -> Result<(), Error> {
            let mut request =
                inlet::Request::new(Operation::Ext(ExtOperation::Port(PortOperation {
                    port: "hello-world".to_string(),
                    entity: Entity::Empty,
                })));

            // send this request to itself
            request.to.push( Identifier::Key(self.inlet_api.info.key.clone()) );

            let response = self.inlet_api.exchange(request).await?;

            if let ResponseEntity::Ok(Entity::Payload(Payload::Text(text))) = response.signal {
                println!("{}",text);
            } else {
                return Err(anyhow!("unexpected signal"));
            }

            Ok(())
        }


    }

    impl HelloCtrl {
        fn hello_world( &self, request: Request<PortOperation> ) {

        }
    }
}
