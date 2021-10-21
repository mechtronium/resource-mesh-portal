use std::collections::HashMap;
use std::convert::From;
use std::convert::TryInto;
use std::sync::Arc;

use anyhow::Error;
use serde::{Deserialize, Serialize};

pub type ExchangeId =String;
pub type BinSrc=String;
pub type BinRaw=Arc<Vec<u8>>;
pub type State=HashMap<String,Bin>;
pub type Key=String;
pub type Address=String;
pub type CliId=String;
pub type ArtifactRef=String;
pub type Artifact=Arc<Vec<u8>>;
pub type Port=String;


pub struct PrimitiveFrame{
    pub data: Vec<u8>
}

impl PrimitiveFrame {
    pub fn size(&self) -> u32 {
        self.data.len() as u32
    }
}

impl From<String> for PrimitiveFrame {
    fn from(value: String) -> Self {
        let bytes = value.as_bytes();
        Self {
            data: bytes.to_vec()
        }
    }


}

impl TryInto<String> for PrimitiveFrame {
    type Error = Error;

    fn try_into(self) -> Result<String, Self::Error> {
        Ok(String::from_utf8(self.data)?)
    }
}


pub enum IdentifierKind {
    Key,
    Address
}

#[derive(Debug,Clone,Serialize,Deserialize,Eq,PartialEq,Hash)]
pub enum Identifier {
    Key(Key),
    Address(Address)
}

#[derive(Debug,Clone,Serialize,Deserialize,Eq,PartialEq,Hash)]
pub struct Identifiers {
  pub key: Key,
  pub address: Address
}

#[derive(Debug,Clone,Serialize,Deserialize,strum_macros::Display)]
pub enum Status {
 Unknown,
 Initializing,
 Ready,
 Panic(String),
 Done
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub enum Log{
    Warn(String),
    Info(String),
    Error(String),
    Fatal(String)
}

impl ToString for Log {
    fn to_string(&self) -> String {
        match self {
            Log::Warn(message) => {format!("WARN: {}",message)}
            Log::Info(message) => {format!("INFO: {}",message)}
            Log::Error(message) => {format!("ERROR: {}",message)}
            Log::Fatal(message) => {format!("FATAL: {}",message)}
        }
    }
}

#[derive(Debug,Clone,Serialize,Deserialize,strum_macros::Display)]
pub enum CloseReason {
    Done,
    Error(String),
}

pub mod resource {
    use serde::{Deserialize, Serialize};

    use crate::version::v0_0_1::{Address, Identifier, Key, State};

    #[derive(Debug,Clone, Serialize, Deserialize)]
    pub struct Archetype {
        pub kind: String,
        pub specific: Option<String>,
        pub config_src: Option<String>
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum ResourceEntity {
        None,
        Resource(ResourceStub),
        Resources(Vec<ResourceStub>),
        State(State)
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ResourceStub {
        pub id: Identifier,
        pub key: Option<Key>,
        pub address: Option<Address>,
        pub archetype: Archetype
    }


}

#[derive(Debug,Clone,Serialize, Deserialize)]
pub enum ExtOperation {
    Http(http::HttpRequest),
    Port(PortRequest)
}

#[derive(Debug,Clone,Serialize, Deserialize)]
pub struct PortRequest {
   pub port: String,
   pub entity: Entity
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub enum Bin {
    Raw(BinRaw),
    Src(BinSrc)
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub enum Payload {
    Text(String),
    Bin(Bin),
    Bins(HashMap<String,Bin>)
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub enum Entity {
    Empty,
    Resource(resource::ResourceEntity),
    Payload(Payload),
    HttpResponse(http::HttpResponse)
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub enum Signal {
    Ok(Entity),
    Error(String)
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub enum ExchangeKind {
    None,
    Notification,
    RequestResponse(ExchangeId)
}

impl ExchangeKind {
    pub fn is_singular_recipient(&self) -> bool {
        match self {
            ExchangeKind::None => false,
            ExchangeKind::Notification => false,
            ExchangeKind::RequestResponse(_) => true
        }
    }
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct BinParcel{
    pub src: BinSrc,
    pub index: u32,
    pub raw: BinRaw
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct Command{
    pub cli: CliId,
    pub payload: String
}

pub mod config {
    use std::collections::HashMap;

    use serde::{Deserialize, Serialize};

    use crate::version::v0_0_1::{Address, ArtifactRef, Identifier, Identifiers, Key};
    use crate::version::v0_0_1::resource::Archetype;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum PortalKind {
        Mechtron,
        Portal
    }

    impl ToString for PortalKind {
        fn to_string(&self) -> String {
            match self {
                PortalKind::Mechtron => "Mechtron".to_string(),
                PortalKind::Portal => "Portal".to_string()
            }
        }
    }


    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Info {
        pub key: Key,
        pub address: Address,
        pub parent: Identifier,
        pub archetype: Archetype,
        pub config: Config,
        pub ext_config: Option<ArtifactRef>,
        pub kind: PortalKind
    }

    impl Info {
        pub fn identity(&self) -> Identifiers {
            Identifiers {
                key: self.key.clone(),
                address: self.address.clone()
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Config {
        pub max_bin_size: u32,
        pub bin_parcel_size: u32,
        pub init_timeout: u64,
        pub frame_timeout: u64,
        pub response_timeout: u64,
        pub bind: BindConfig
    }

    impl Config {
        pub fn with_bind_config( bind: BindConfig ) -> Self {
            Self {
                max_bin_size: 128*1024,
                bin_parcel_size: 16*1024,
                init_timeout: 30,
                frame_timeout: 5,
                response_timeout: 15,
                bind
            }
        }
    }

    impl Default for Config {
        fn default() -> Self {
            Self {
                max_bin_size: 128*1024,
                bin_parcel_size: 16*1024,
                init_timeout: 30,
                frame_timeout: 5,
                response_timeout: 15,
                bind: Default::default()
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SchemaRef {
       pub schema: String,
       pub artifact: Option<ArtifactRef>
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BindConfig {
        pub ports: HashMap<String, PortConfig>,
    }

    impl Default for BindConfig {
        fn default() -> Self {
            Self {
                ports: Default::default()
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PortConfig {
        pub payload: PayloadConfig,
        pub response: EntityConfig
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum EntityConfig {
        Empty,
        Resource(ResourceConfig),
        Payload(PayloadConfig),
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum ResourceConfig {
        None,
        Resource,
        Resources,
        State
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum PayloadConfig {
        Text,
        Bin(SchemaRef),
        Bins(HashMap<String, SchemaRef>)
    }


}

pub mod http{
    use std::collections::HashMap;

    use serde::{Deserialize, Serialize};

    use crate::version::v0_0_1::Bin;

    #[derive(Debug,Clone,Serialize,Deserialize)]
    pub struct HttpRequest {
        pub headers: HashMap<String,String>,
        pub path: String,
        pub body: Option<Bin>
    }

    #[derive(Debug,Clone,Serialize,Deserialize)]
    pub struct HttpResponse{
        pub headers: HashMap<String,String>,
        pub code: usize,
        pub body: Option<Bin>
    }

    impl HttpResponse {
        pub fn server_side_error() -> Self {
            Self {
                headers: Default::default(),
                code: 500,
                body: None
            }
        }
    }
}

pub mod mesh {
    pub mod inlet {
        use std::convert::TryFrom;
        use std::convert::TryInto;

        use anyhow::Error;
        use serde::{Deserialize, Serialize};

        use crate::version::v0_0_1::{BinParcel, Command, ExchangeId, ExchangeKind, Identifier, Log, PrimitiveFrame, Signal, Status, CloseReason};
        use crate::version::v0_0_1::mesh::inlet::resource::Operation;

        #[derive(Debug,Clone,Serialize,Deserialize)]
        pub struct Request {
            pub to: Vec<Identifier>,
            pub operation: Operation,
            pub kind: ExchangeKind,
        }

        impl Request {
            pub fn new(operation: Operation) -> Self {
                Self{
                    to: vec![],
                    operation,
                    kind: ExchangeKind::None
                }
            }
        }

        #[derive(Debug,Clone,Serialize,Deserialize)]
        pub struct Response {
            pub to: Identifier,
            pub exchange_id: ExchangeId,
            pub signal: Signal,
        }

        #[derive(Debug,Clone,Serialize,Deserialize,strum_macros::Display)]
        pub enum Frame {
            Log(Log),
            Command(Command),
            Request(Request),
            Response(Response),
            Status(Status),
            BinParcel(BinParcel),
            Close(CloseReason)
        }

        impl TryInto<PrimitiveFrame> for Frame {
            type Error = Error;

            fn try_into(self) -> Result<PrimitiveFrame, Self::Error> {
                let data = bincode::serialize(&self)?;
                Ok( PrimitiveFrame {
                    data
                })
            }
        }

        impl TryFrom<PrimitiveFrame> for Frame {
            type Error = Error;

            fn try_from(value: PrimitiveFrame) -> Result<Self, Self::Error> {
                let frame = bincode::deserialize(value.data.as_slice() )?;
                Ok(frame)
            }
        }

        pub mod resource {
            use serde::{Deserialize, Serialize};

            use crate::version::v0_0_1::{ExtOperation, Identifier, State};
            use crate::version::v0_0_1::resource::Archetype;

            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub enum Operation {
                Resource(ResourceOperation),
                Ext(ExtOperation)
            }

            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub enum ResourceOperation {
                Create(Create),
                Select(Selector),
                Get,
                Set(State),
                Delete,
            }

            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub struct Create {
                pub parent: Identifier,
                pub archetype: Archetype,
                pub address: AddressSrc,
                pub strategy: CreateStrategy,
                pub state: StateSrc,
            }

            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub enum StateSrc {
                Stateless,
                State(State),
                CreateArgs(String),
            }

            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub enum CreateStrategy {
                Create,
                CreateOrUpdate,
                Ensure,
            }

            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub enum AddressSrc {
                Append(String),
                Pattern(String)
            }


            #[derive(Debug,Clone, Serialize, Deserialize)]
            pub struct Selector {
                meta: MetaSelector
            }

            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub enum MetaSelector {
                None,
                Name(String)
            }
        }


    }

    pub mod outlet {
        use std::convert::TryFrom;
        use std::convert::TryInto;

        use anyhow::Error;
        use serde::{Deserialize, Serialize};

        use crate::version::v0_0_1::{BinParcel, CliId, Entity, ExchangeId, ExchangeKind, ExtOperation, Identifier, Port, PrimitiveFrame, Signal, CloseReason};
        use crate::version::v0_0_1::config::Info;

        #[derive(Debug,Clone,Serialize,Deserialize)]
        pub struct Request {
            pub from: Identifier,
            pub operation: ExtOperation,
            pub kind: ExchangeKind,
        }

        #[derive(Debug,Clone,Serialize,Deserialize)]
        pub struct Response {
            pub from: Identifier,
            pub exchange_id: ExchangeId,
            pub signal: Signal,
        }

        #[derive(Debug,Clone,Serialize,Deserialize)]
        pub struct CommandEvent{
            pub cli: CliId,
            pub line: Option<String>,
            pub status: CommandStatus
        }

        #[derive(Debug,Clone,Serialize,Deserialize)]
        pub enum CommandStatus{
            Running,
            Exit(i32)
        }



        #[derive(Debug,Clone,Serialize,Deserialize,strum_macros::Display)]
        pub enum Frame {
            Init(Info),
            CommandEvent(CommandEvent),
            Request(Request),
            Response(Response),
            BinParcel(BinParcel),
            Close(CloseReason)
        }

        impl TryInto<PrimitiveFrame> for Frame {
            type Error = Error;

            fn try_into(self) -> Result<PrimitiveFrame, Self::Error> {
                let data = bincode::serialize(&self)?;
                Ok( PrimitiveFrame {
                    data
                })
            }
        }

        impl TryFrom<PrimitiveFrame> for Frame {
            type Error = Error;

            fn try_from(value: PrimitiveFrame) -> Result<Self, Self::Error> {
                let frame = bincode::deserialize(value.data.as_slice() )?;
                Ok(frame)
            }
        }

        pub mod resource {
            use serde::{Deserialize, Serialize};

            use crate::version::v0_0_1::resource::ResourceStub;
            use crate::version::v0_0_1::State;

            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub enum ResourceEntity {
                None,
                Resource(ResourceStub),
                Resources(Vec<ResourceStub>),
                State(State)
            }
        }
    }
}




