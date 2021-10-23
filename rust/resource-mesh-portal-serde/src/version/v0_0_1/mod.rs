use std::collections::HashMap;
use std::convert::From;
use std::convert::TryInto;
use std::sync::Arc;

use anyhow::Error;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use std::fmt::Debug;
use crate::version::v0_0_1::generic::Bin;
use crate::version::v0_0_1::bin::Bin;


pub type State=HashMap<String,Bin>;

pub type ArtifactRef=String;
pub type Artifact=Arc<Vec<u8>>;
pub type Port=String;

pub mod id {
    pub type Key=String;
    pub type Address=String;
    pub type Kind=String;

    pub enum IdentifierKind {
        Key,
        Address
    }

    pub type Identifiers = generic::Identifiers<Key,Address>;
}


pub mod messaging {
    pub type ExchangeId = String;

    #[derive(Debug, Clone, Serialize, Deserialize)]
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
}


pub mod log {
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum Log {
        Warn(String),
        Info(String),
        Error(String),
        Fatal(String)
    }

    impl ToString for Log {
        fn to_string(&self) -> String {
            match self {
                Log::Warn(message) => { format!("WARN: {}", message) }
                Log::Info(message) => { format!("INFO: {}", message) }
                Log::Error(message) => { format!("ERROR: {}", message) }
                Log::Fatal(message) => { format!("FATAL: {}", message) }
            }
        }
    }
}

pub mod frame {
    use std::convert::TryInto;
    use anyhow::Error;

    pub struct PrimitiveFrame {
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

    #[derive(Debug, Clone, Serialize, Deserialize, strum_macros::Display)]
    pub enum CloseReason {
        Done,
        Error(String),
    }
}

pub mod bin {
    use std::sync::Arc;

    pub type BinSrc=String;
    pub type BinRaw=Arc<Vec<u8>>;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum Bin {
        Raw(BinRaw),
        Src(BinSrc)
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BinParcel {
        pub src: BinSrc,
        pub index: u32,
        pub raw: BinRaw
    }
}

pub mod delivery {
    use crate::version::v0_0_1::bin::Bin;
    use std::collections::HashMap;
    use crate::version::v0_0_1::id::{Key, Address, Kind};
    use crate::version::v0_0_1::generic;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum Payload {
        Text(String),
        Bin(Bin),
        Bins(HashMap<String, Bin>)
    }

    pub type Entity = generic::delivery::Entity<Key,Address,Kind>;
    pub type ResourceEntity = generic::delivery::ResourceEntity<Key,Address,Kind>;
    pub type ResponseSignal = generic::delivery::ResponseEntity<Key,Address,Kind>;
}

pub mod command {

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Command {
        pub cli: CliId,
        pub payload: String
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum CommandStatus {
        Running,
        Exit(i32)
    }

    pub type CliId=String;
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CommandEvent {
        pub cli: CliId,
        pub line: Option<String>,
        pub status: CommandStatus
    }
}

pub mod http {
    use std::collections::HashMap;

    use serde::{Deserialize, Serialize};

    use crate::version::v0_0_1::Bin;
    use crate::version::v0_0_1::generic::Bin;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct HttpRequest {
        pub headers: HashMap<String, String>,
        pub path: String,
        pub body: Option<Bin>
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct HttpResponse {
        pub headers: HashMap<String, String>,
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


pub mod config {
    use crate::version::v0_0_1::id::{Key, Address, Kind};

    use crate::version::v0_0_1::{generic, Kind, ArtifactRef};
    use std::collections::HashMap;

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

    pub type Info = generic::config::Info<Key,Address,Kind>;


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
        pub fn with_bind_config(bind: BindConfig) -> Self {
            Self {
                max_bin_size: 128 * 1024,
                bin_parcel_size: 16 * 1024,
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
                max_bin_size: 128 * 1024,
                bin_parcel_size: 16 * 1024,
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

pub mod operation {

    use crate::version::v0_0_1::id::{Key, Address, Kind};

    use crate::version::v0_0_1::{Generic, State, http};
    use crate::version::v0_0_1::generic::resource::{Create, Selector};
    use crate::version::v0_0_1::generic::operation;

    pub type Operation = operation::Operation<Key,Address,Kind>;
    pub type ResourceOperation = operation::ResourceOperation<Key,Address,Kind>;
    pub type ExtOperation = operation::ExtOperation<Key,Address,Kind>;
    pub type PortOperation = operation::PortOperation<Key,Address,Kind>;
}

pub mod resource {
    use crate::version::v0_0_1::id::{Key, Address, Kind};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, strum_macros::Display)]
    pub enum Status {
        Unknown,
        Initializing,
        Ready,
        Panic(String),
        Done
    }

    use crate::version::v0_0_1::{ExtOperation, Identifier, State, generic, Key, Address, Kind};
    use crate::version::v0_0_1::generic::{ExtOperation, Generic, Identifier};
    use crate::version::v0_0_1::generic::resource::Archetype;

    pub type Operation=generic::resource::Operation<Key,Address,Kind>;
    pub type ResourceOperation=generic::resource::ResourceOperation<Key,Address,Kind>;
    pub type Create=generic::resource::Create<Key,Address,Kind>;

    pub type StateSrc=generic::resource::StateSrc;
    pub type CreateStrategy=generic::resource::CreateStrategy;
    pub type AddressSrc=generic::resource::AddressSrc;
    pub type Selector=generic::resource::Selector;
    pub type MetaSelector=generic::resource::MetaSelector;
}

pub mod portal {
    pub mod inlet {
        use crate::version::v0_0_1::id::{Key, Address, Kind};
        use std::convert::TryFrom;
        use std::convert::TryInto;

        use anyhow::Error;
        use serde::{Deserialize, Serialize};

        use crate::version::v0_0_1::{BinParcel, Command, ExchangeId, ExchangeKind, Identifier, Log, PrimitiveFrame, ResponseSignal, Status, CloseReason, generic, Key, Address, Kind};
        use crate::version::v0_0_1::generic::{Identifier, ExchangeKind, Generic, ResponseSignal, Log, Command, Status, BinParcel, CloseReason, PrimitiveFrame};

        pub type Request=generic::portal::inlet::Request<Key,Address,Kind>;
        pub type Response=generic::portal::inlet::Response<Key,Address,Kind>;
        pub type Frame=generic::portal::inlet::Frame<Key,Address,Kind>;
    }

    pub mod outlet {
        use crate::version::v0_0_1::id::{Key, Address, Kind};
        use std::convert::TryFrom;
        use std::convert::TryInto;

        use anyhow::Error;
        use serde::{Deserialize, Serialize};

        use crate::version::v0_0_1::{BinParcel, CliId, Entity, ExchangeId, ExchangeKind, ExtOperation, Identifier, Port, PrimitiveFrame, ResponseSignal, CloseReason, Key, Address, Kind, generic};
        use crate::version::v0_0_1::generic::{Identifier, ExtOperation, ExchangeKind, Generic, ResponseSignal, BinParcel, CloseReason, PrimitiveFrame};

        pub type Request=generic::portal::outlet::Request<Key,Address,Kind>;
        pub type Response=generic::portal::outlet::Response<Key,Address,Kind>;
        pub type Frame=generic::portal::outlet::Frame<Key,Address,Kind>;
        pub type CommandEvent=generic::portal::outlet::CommandEvent;
    }
}

pub mod generic {
    use std::collections::HashMap;
    use std::convert::From;
    use std::convert::TryInto;
    use std::sync::Arc;

    use anyhow::Error;
    use serde::{Deserialize, Serialize};
    use std::hash::Hash;
    use std::fmt::Debug;
    use crate::version::v0_0_1::{ExchangeId, BinSrc, BinRaw, CliId, Generic, Payload, http};
    use crate::version::v0_0_1::generic::delivery::Entity;

    pub trait Generic: Debug + Clone + Serialize + Deserialize + Eq + PartialEq + Hash + Send + Sync {}

    pub mod id {
        use crate::version::v0_0_1::generic::Generic;

        #[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
        pub enum Identifier<KEY: Generic, ADDRESS: Generic> {
            Key(KEY),
            Address(ADDRESS)
        }

        #[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
        pub struct Identifiers<KEY: Generic, ADDRESS: Generic> {
            pub key: KEY,
            pub address: ADDRESS
        }
    }




    pub mod config {
        use crate::version::v0_0_1::generic::{Identifier, Identifiers, Generic};
        use crate::version::v0_0_1::generic::resource::Archetype;
        use crate::version::v0_0_1::config::{PortalKind, Config};
        use crate::version::v0_0_1::generic::id::{Identifier, Identifiers};
        use crate::version::v0_0_1::ArtifactRef;


        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct Info<KEY: Generic, ADDRESS: Generic, KIND: Generic> {
            pub key: KEY,
            pub address: ADDRESS,
            pub owner: String,
            pub parent: Identifier<KEY,ADDRESS>,
            pub archetype: Archetype<KIND>,
            pub config: Config,
            pub ext_config: Option<ArtifactRef>,
            pub kind: PortalKind
        }

        impl <KEY: Generic, ADDRESS: Generic, KIND: Generic> Info<KEY, ADDRESS, KIND> {
            pub fn identity(&self) -> Identifiers<KEY,ADDRESS> {
                Identifiers {
                    key: self.key.clone(),
                    address: self.address.clone()
                }
            }
        }
    }

    pub mod operation {
        use crate::version::v0_0_1::{Generic, State, http};
        use crate::version::v0_0_1::generic::resource::{Create, Selector};
        use crate::version::v0_0_1::generic::delivery::Entity;
        use crate::version::v0_0_1::generic::Generic;

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub enum Operation<KEY: Generic, ADDRESS: Generic, KIND: Generic>  {
            Resource(ResourceOperation<KEY,ADDRESS,KIND>),
            Ext(ExtOperation<KEY,ADDRESS,KIND>)
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub enum ResourceOperation<KEY: Generic, ADDRESS: Generic, KIND: Generic>  {
            Create(Create<KEY,ADDRESS,KIND>),
            Select(Selector),
            Get,
            Set(State),
            Delete,
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub enum ExtOperation<KEY: Generic, ADDRESS: Generic, KIND: Generic> {
            Http(http::HttpRequest),
            Port(PortOperation<KEY,ADDRESS,KIND>)
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct PortOperation<KEY: Generic, ADDRESS: Generic, KIND: Generic> {
            pub port: String,
            pub entity: Entity<KEY,ADDRESS,KIND>
        }
    }


    pub mod resource {
        use serde::{Deserialize, Serialize};
        use crate::version::v0_0_1::{Generic, State};
        use crate::version::v0_0_1::generic::{Identifier, Generic};
        use crate::version::v0_0_1::generic::id::Identifier;

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct Archetype<KIND: Generic> {
            pub kind: KIND,
            pub specific: Option<String>,
            pub config_src: Option<String>
        }


        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct ResourceStub<KEY: Generic, ADDRESS: Generic, KIND: Generic> {
            pub id: Identifier<KEY,ADDRESS>,
            pub key: KEY,
            pub address: ADDRESS,
            pub archetype: Archetype<KIND>
        }



        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct Create<KEY: Generic, ADDRESS: Generic, KIND: Generic> {
            pub parent: Identifier<KEY,ADDRESS>,
            pub archetype: Archetype<KIND>,
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


        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct Selector {
            meta: MetaSelector
        }

        impl Selector {
            pub fn new() -> Self {
                Self {
                    meta: MetaSelector::None
                }
            }
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub enum MetaSelector {
            None,
            Name(String)
        }
    }

    pub mod portal {
        pub mod inlet {
            use std::convert::TryFrom;
            use std::convert::TryInto;

            use anyhow::Error;
            use serde::{Deserialize, Serialize};

            use crate::version::v0_0_1::{BinParcel, Command, ExchangeId, ExchangeKind, Identifier, Log, PrimitiveFrame, ResponseSignal, Status, CloseReason};
            use crate::version::v0_0_1::generic::{Identifier, ExchangeKind, Generic, ResponseSignal, Log, Command, Status, BinParcel, CloseReason, PrimitiveFrame};
            use crate::version::v0_0_1::generic::portal::inlet::resource::Operation;
            use crate::version::v0_0_1::generic::delivery::ResponseEntity;
            use crate::version::v0_0_1::log::Log;
            use crate::version::v0_0_1::command::Command;
            use crate::version::v0_0_1::bin::BinParcel;
            use crate::version::v0_0_1::frame::{CloseReason, PrimitiveFrame};
            use crate::version::v0_0_1::resource::Status;
            use crate::version::v0_0_1::generic::operation::Operation;
            use crate::version::v0_0_1::generic::id::Identifier;
            use crate::version::v0_0_1::messaging::{ExchangeKind, ExchangeId};

            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub struct Request<KEY: Generic, ADDRESS: Generic, KIND: Generic>  {
                pub to: Vec<Identifier<KEY,ADDRESS>>,
                pub operation: Operation<KEY,ADDRESS,KIND>,
                pub kind: ExchangeKind,
            }

            impl <KEY: Generic, ADDRESS: Generic, KIND: Generic> Request<KEY,ADDRESS,KIND> {
                pub fn new(operation: Operation<KEY,ADDRESS,KIND>) -> Self {
                    Self {
                        to: vec![],
                        operation,
                        kind: ExchangeKind::None
                    }
                }
            }

            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub struct Response<KEY: Generic, ADDRESS: Generic, KIND: Generic> {
                pub to: Identifier<KEY,ADDRESS>,
                pub exchange_id: ExchangeId,
                pub signal: ResponseEntity<KEY,ADDRESS,KIND>,
            }

            #[derive(Debug, Clone, Serialize, Deserialize, strum_macros::Display)]
            pub enum Frame<KEY: Generic, ADDRESS: Generic, KIND: Generic> {
                Log(Log),
                Command(Command),
                Request(Request<KEY,ADDRESS,KIND>),
                Response(Response<KEY,ADDRESS,KIND>),
                Status(Status),
                BinParcel(BinParcel),
                Close(CloseReason)
            }

            impl <KEY: Generic, ADDRESS: Generic, KIND: Generic> TryInto<PrimitiveFrame> for Frame<KEY,ADDRESS,KIND> {
                type Error = Error;

                fn try_into(self) -> Result<PrimitiveFrame, Self::Error> {
                    let data = bincode::serialize(&self)?;
                    Ok(PrimitiveFrame {
                        data
                    })
                }
            }

            impl <KEY: Generic, ADDRESS: Generic, KIND: Generic> TryFrom<PrimitiveFrame> for Frame<KEY,ADDRESS,KIND>{
                type Error = Error;

                fn try_from(value: PrimitiveFrame) -> Result<Self, Self::Error> {
                    let frame = bincode::deserialize(value.data.as_slice())?;
                    Ok(frame)
                }
            }
        }

        pub mod outlet {
            use std::convert::TryFrom;
            use std::convert::TryInto;

            use anyhow::Error;
            use serde::{Deserialize, Serialize};

            use crate::version::v0_0_1::{BinParcel, CliId, Entity, ExchangeId, ExchangeKind, ExtOperation, Identifier, Port, PrimitiveFrame, ResponseSignal, CloseReason};
            use crate::version::v0_0_1::generic::{Identifier, ExtOperation, ExchangeKind, Generic, ResponseSignal, BinParcel, CloseReason, PrimitiveFrame};
            use crate::version::v0_0_1::generic::config::Info;
            use crate::version::v0_0_1::frame::{CloseReason, PrimitiveFrame};
            use crate::version::v0_0_1::generic::delivery::ResponseEntity;
            use crate::version::v0_0_1::command::CommandEvent;
            use crate::version::v0_0_1::bin::BinParcel;
            use crate::version::v0_0_1::generic::operation::ExtOperation;
            use crate::version::v0_0_1::generic::id::Identifier;
            use crate::version::v0_0_1::messaging::{ExchangeKind, ExchangeId};

            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub struct Request<KEY: Generic, ADDRESS: Generic, KIND: Generic> {
                pub from: Identifier<KEY,ADDRESS>,
                pub operation: ExtOperation<KEY,ADDRESS,KIND>,
                pub kind: ExchangeKind
            }

            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub struct Response<KEY: Generic, ADDRESS: Generic, KIND: Generic> {
                pub from: Identifier<KEY,ADDRESS>,
                pub exchange_id: ExchangeId,
                pub signal: ResponseEntity<KEY,ADDRESS,KIND>,
            }

            #[derive(Debug, Clone, Serialize, Deserialize, strum_macros::Display)]
            pub enum Frame<KEY: Generic, ADDRESS: Generic, KIND: Generic> {
                Init(Info<KEY,ADDRESS,KIND>),
                CommandEvent(CommandEvent),
                Request(Request<KEY,ADDRESS,KIND>),
                Response(Response<KEY,ADDRESS,KIND>),
                BinParcel(BinParcel),
                Close(CloseReason)
            }

            impl <KEY: Generic, ADDRESS: Generic, KIND: Generic> TryInto<PrimitiveFrame> for Frame<KEY,ADDRESS,KIND> {
                type Error = Error;

                fn try_into(self) -> Result<PrimitiveFrame, Self::Error> {
                    let data = bincode::serialize(&self)?;
                    Ok(PrimitiveFrame {
                        data
                    })
                }
            }

            impl <KEY: Generic, ADDRESS: Generic, KIND: Generic> TryFrom<PrimitiveFrame> for Frame<KEY,ADDRESS,KIND> {
                type Error = Error;

                fn try_from(value: PrimitiveFrame) -> Result<Self, Self::Error> {
                    let frame = bincode::deserialize(value.data.as_slice())?;
                    Ok(frame)
                }
            }
        }
    }

    pub mod delivery {
        use serde::{Deserialize, Serialize};

        use crate::version::v0_0_1::resource::ResourceStub;
        use crate::version::v0_0_1::{State, http};
        use crate::version::v0_0_1::generic::Generic;
        use crate::version::v0_0_1::generic::resource::ResourceStub;
        use crate::version::v0_0_1::delivery::Payload;

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub enum Entity<KEY: Generic, ADDRESS: Generic, KIND: Generic> {
            Empty,
            Resource(ResourceEntity<KEY,ADDRESS,KIND>),
            Payload(Payload),
            HttpResponse(http::HttpResponse)
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub enum ResourceEntity<KEY: Generic, ADDRESS: Generic, KIND: Generic> {
            None,
            Stub(ResourceStub<KEY,ADDRESS,KIND>),
            Stubs(Vec<ResourceStub<KEY,ADDRESS,KIND>>),
            State(State)
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub enum ResponseEntity<KEY: Generic, ADDRESS: Generic, KIND: Generic> {
            Ok(Entity<KEY,ADDRESS,KIND>),
            Error(String)
        }

    }
}