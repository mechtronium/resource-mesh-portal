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

#[derive(Debug,Clone,Serialize,Deserialize)]
pub enum Status {
 Unknown,
 Initializing,
 Ready,
 Panic(String)
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
    Resource(mesh::enter::Operation),
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
    Notification,
    RequestResponse(ExchangeId)
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
    
    use crate::ArtifactRef;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Config {
        pub max_bin_size: u32,
        pub bin_parcel_size: u32
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
    pub mod enter {
        use serde::{Serialize,Deserialize};

        use crate::config::BindConfig;
        use crate::{Identifier, Operation, ExchangeKind, ExchangeId, Signal, CliId, Command, Status, BinParcel};

        #[derive(Debug,Clone,Serialize,Deserialize)]
        pub struct Request {
            pub to: Vec<Identifier>,
            pub operation: Operation,
            pub kind: ExchangeKind,
        }

        #[derive(Debug,Clone,Serialize,Deserialize)]
        pub struct Response {
            pub to: Identifier,
            pub exchange_id: ExchangeId,
            pub signal: Signal,
        }

        #[derive(Debug,Clone,Serialize,Deserialize)]
        pub enum Frame {
            StartCli(CliId),
            Command(Command),
            EndCli(CliId),
            Request(Request),
            Response(Response),
            GetBindConfig,
            SetBindConfig(BindConfig),
            SetStatus(Status),
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

    pub mod exit {
        use serde::{Serialize,Deserialize};


        use crate::config::BindConfig;
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
        pub struct CommandOut{
            pub cli: CliId,
            pub payload: String
        }

        #[derive(Debug,Clone,Serialize,Deserialize)]
        pub enum Frame {
            StartCli(CliId),
            Command(CommandOut),
            EndCli(CliId),
            Request(Request),
            Response(Response),
            BindConfig(BindConfig),
            BinParcel(BinParcel)
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
