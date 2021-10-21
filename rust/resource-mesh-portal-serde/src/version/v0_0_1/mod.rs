use std::collections::HashMap;
use std::sync::Arc;
use serde::{Serialize,Deserialize};

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
 Panic(String)
}

#[derive(Debug,Clone,Serialize,Deserialize)]
pub enum Log{
    Warn(String),
    Info(String),
    Error(String),
    Fatal(String)
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

    use crate::version::v0_0_1::{Address, ArtifactRef, Identifier, Key, Identifiers};
    use crate::version::v0_0_1::resource::Archetype;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Info {
        pub key: Key,
        pub address: Address,
        pub parent: Identifier,
        pub archetype: Archetype,
        pub config: Config,
        pub ext_config: Option<ArtifactRef>
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

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SchemaRef {
       pub schema: String,
       pub artifact: Option<ArtifactRef>
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct BindConfig {
        pub ports: HashMap<String, PortConfig>,
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
        use serde::{Deserialize, Serialize};

        use crate::version::v0_0_1::{BinParcel, Command, ExchangeId, ExchangeKind, Identifier, Log, Signal, Status};
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
            BinParcel(BinParcel)
        }

        pub mod resource {
            use serde::{Deserialize, Serialize};

            use crate::version::v0_0_1::{Identifier, State, ExtOperation};
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
        use serde::{Deserialize, Serialize};

        use crate::version::v0_0_1::{BinParcel, CliId, Entity, ExchangeId, ExchangeKind, Identifier, Port, Signal, ExtOperation};
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
            Shutdown
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
