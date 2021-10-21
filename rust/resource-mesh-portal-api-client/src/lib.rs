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

use resource_mesh_portal_serde::version::v0_0_1::{Entity, ExchangeId, ExchangeKind, Identifier, Log, mesh, Port, Signal, Status, ExtOperation, PortRequest};
use resource_mesh_portal_serde::version::v0_0_1::config::Info;
use resource_mesh_portal_serde::version::v0_0_1::mesh::outlet::Frame;
use std::prelude::rust_2021::TryFrom;
use std::ops::Deref;
use resource_mesh_portal_serde::version::v0_0_1::http::{HttpRequest, HttpResponse};
use std::collections::HashMap;
use resource_mesh_portal_serde::version::v0_0_1::mesh::inlet::Response;
use tokio::sync::watch::Receiver;


#[async_trait]
pub trait PortalCtrl : Sync+Send {
    async fn init(&mut self) -> Result<(), Error>
    {
        Ok(())
    }

    fn ports(&self) -> HashMap<String,fn( request: Request<PortRequest> ) -> Result<Option<mesh::inlet::Response>,Error>> {
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
    pub inlet_api: Arc<InletApi>,
    pub ports: HashMap<String,fn( request: Request<PortRequest> ) -> Result<Option<mesh::inlet::Response>,Error>>,
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
        let ports = ctrl.ports();
        ctrl.init().await?;
        let ctrl = ctrl.into();
        let inlet_api = Arc::new(InletApi::new(skel.clone() ));
        let portal = Self {
            skel: skel.clone(),
            ctrl,
            inlet_api,
            ports
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
                                                    let response = mesh::inlet::Response {
                                                        to: from,
                                                        exchange_id:exchange_id.clone(),
                                                        signal: Signal::Ok(Entity::HttpResponse(response))
                                                    };
                                                    inlet_api.respond( response );
                                                }
                                                Err(err) => {
                                                    (skel.logger)(format!("ERROR: HttpRequest.path: '{}' error: '{}' ",  path, err.to_string()).as_str());
                                                    let response = mesh::inlet::Response {
                                                        to: from,
                                                        exchange_id:exchange_id.clone(),
                                                        signal: Signal::Ok(Entity::HttpResponse(HttpResponse::server_side_error()))
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
                                                let result = (*port)(request);
                                                match result {
                                                    Ok(response) => {
                                                        match response {
                                                            Some(response) => {
                                                                if let ExchangeKind::RequestResponse(exchange_id) = &kind
                                                                {
                                                                   inlet_api.respond(response);
                                                                } else {
                                                                    let message = format!("WARN: PortRequest.port '{}' generated a response to a ExchangeKind::Notification", port_request.port);
                                                                    (skel.logger)(message.as_str());
                                                                }
                                                            }
                                                            None => {
                                                                let message = format!("ERROR: PortRequest.port '{}' generated no response", port_request.port);
                                                                (skel.logger)(message.as_str());
                                                                if let ExchangeKind::RequestResponse(exchange_id) = &kind
                                                                {
                                                                    let response = mesh::inlet::Response {
                                                                        to: from,
                                                                        exchange_id: exchange_id.clone(),
                                                                        signal: Signal::Error(message)
                                                                    };
                                                                    inlet_api.respond(response);
                                                                }
                                                            }
                                                        }
                                                    }
                                                    Err(err) => {
                                                        let message = format!("ERROR: PortRequest.port '{}' message: '{}'", port_request.port, err.to_string());
                                                        (skel.logger)(message.as_str());
                                                        if let ExchangeKind::RequestResponse(exchange_id) = &kind
                                                        {
                                                            let response = mesh::inlet::Response {
                                                                to: from,
                                                                exchange_id: exchange_id.clone(),
                                                                signal: Signal::Error(message)
                                                            };
                                                            inlet_api.respond(response);
                                                        }
                                                    }
                                                }

                                            }
                                            Err(err) => {
                                                let message = format!("FATAL: could not modify PortRequest into Request<PortRequest>: {}", err.to_string());
                                                (skel.logger)(message.as_str());
                                                if let ExchangeKind::RequestResponse(exchange_id) = &kind
                                                {
                                                    let response = mesh::inlet::Response {
                                                        to: from,
                                                        exchange_id: exchange_id.clone(),
                                                        signal: Signal::Error(message)
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
                                            let response = mesh::inlet::Response {
                                                to: from,
                                                exchange_id: exchange_id.clone(),
                                                signal: Signal::Error(message)
                                            };
                                            inlet_api.respond(response);
                                        }
                                    }
                                }
                            }
                        }
                    });
                }
                Frame::Response(response) => {
                    if let Option::Some((_,tx)) =
                        self.skel.exchanges.remove(&response.exchange_id)
                    {
                        tx.send(response).unwrap_or(());
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
        let exchange_id: ExchangeId = Uuid::new_v4().to_string();
        request.kind = ExchangeKind::RequestResponse(exchange_id.clone());
        let (tx,rx) = oneshot::channel();
        self.skel.exchanges.insert(exchange_id, tx);
        self.skel.inlet.send(mesh::inlet::Frame::Request(request));

        let result = tokio::time::timeout(Duration::from_secs(self.skel.info.config.response_timeout.clone()),rx).await;
        Ok(result??)
    }

    pub fn respond( &self, response: mesh::inlet::Response ) {
        self.skel.inlet.send( mesh::inlet::Frame::Response(response) );
    }
}

#[derive(Clone)]
pub struct RequestContext{
    pub portal_info: Info,
    pub logger: fn(message: &str),
}

impl RequestContext {
    pub fn new( portal_info: Info, logger: fn(message: &str) ) -> Self {
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

impl <REQUEST> Deref for Request<REQUEST> {
    type Target = REQUEST;

    fn deref(&self) -> &Self::Target {
        &self.request
    }
}

impl Request<HttpRequest> {
    pub fn try_from_http(request: mesh::outlet::Request, context: RequestContext) -> Result<Request<HttpRequest>, Error> {
        if let ExtOperation::Http(http_request) = request.operation {
            Ok(Self {
                context,
                from: request.from,
                request: http_request,
            })
        } else {
            Err(anyhow!("can only create Request<PortRequest> from ExtOperation::Port"))
        }
    }
}

impl Request<PortRequest> {
    pub fn try_from_port( request: mesh::outlet::Request, context: RequestContext ) -> Result<Request<PortRequest>,Error> {

        if let ExtOperation::Port( port_request ) = request.operation {
            Ok(Self {
                context,
                from: request.from,
                request: port_request,
            })
        } else {
            Err(anyhow!("can only create Request<PortRequest> from ExtOperation::Port"))
        }
    }
}


pub mod example {
    use std::sync::Arc;

    use anyhow::Error;

    use resource_mesh_portal_serde::version::v0_0_1::{Entity, ExtOperation, mesh, Payload, PortRequest, Signal, Identifier};

    use crate::{InletApi, PortalCtrl, PortalSkel, Request};
    use resource_mesh_portal_serde::version::v0_0_1::mesh::inlet::resource::Operation;
    use resource_mesh_portal_serde::version::v0_0_1::http::{HttpRequest, HttpResponse};
    use std::collections::HashMap;

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
                mesh::inlet::Request::new(Operation::Ext(ExtOperation::Port(PortRequest {
                    port: "hello-world".to_string(),
                    entity: Entity::Empty,
                })));

            // send this request to itself
            request.to.push( Identifier::Key(self.inlet_api.skel.info.key.clone()) );

            let response = self.inlet_api.exchange(request).await?;

            if let Signal::Ok(Entity::Payload(Payload::Text(text))) = response.signal {
                println!("{}",text);
            } else {
                return Err(anyhow!("unexpected signal"));
            }

            Ok(())
        }


    }

    impl HelloCtrl {
        fn hello_world( &self, request: Request<PortRequest> ) {

        }
    }
}
