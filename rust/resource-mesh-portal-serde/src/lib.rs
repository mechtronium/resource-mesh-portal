
use std::collections::HashMap;
use std::sync::Arc;
use serde::{Serialize,Deserialize};

pub type Identifier=String;
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
    use serde::{Serialize,Deserialize};
    use crate::{State, Identifier, Key, Address};

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

#[derive(Debug,Clone,Serialize,Deserialize)]
pub enum Operation {
    Resource(mesh::inlet::resource::Operation),
    Ext(ExtOperation)
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
    Payload(Payload)
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
    use serde::{Serialize,Deserialize};
    use std::collections::HashMap;
    
    use crate::{ArtifactRef, Key, Address, Identifier};
    use crate::resource::Archetype;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Info {
        pub key: Key,
        pub address: Address,
        pub parent: Identifier,
        pub archetype: Archetype,
        pub config: Config,
        pub ext_config: Option<ArtifactRef>
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
    use serde::{Serialize,Deserialize};

    use std::collections::HashMap;
    use crate::Bin;

    #[derive(Debug,Clone,Serialize,Deserialize)]
    pub struct HttpRequest {
        pub path: String,
        pub headers: HashMap<String,String>,
        pub body: Bin
    }
}

pub mod mesh {
    pub mod inlet {
        use serde::{Serialize,Deserialize};

        use crate::{Identifier, Operation, ExchangeKind, ExchangeId, Signal, Command, Status, BinParcel, Log};

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
            use serde::{Serialize,Deserialize};
            use crate::{State, Identifier};
            use crate::resource::Archetype;

            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub enum Operation {
                Create(Create),
                Select(Selector),
                Get,
                Set(State),
                Delete
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
        use serde::{Serialize,Deserialize};

        use crate::config::{Info};
        use crate::{Identifier, Entity, ExchangeKind, ExchangeId, Signal, Port, CliId, BinParcel};

        #[derive(Debug,Clone,Serialize,Deserialize)]
        pub struct Request {
            pub from: Identifier,
            pub port: Port,
            pub entity: Entity,
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
            use serde::{Serialize,Deserialize};
            use crate::{State};
            use crate::resource::{ResourceStub};

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








#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
