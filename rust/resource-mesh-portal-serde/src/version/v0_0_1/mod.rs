mod parse;

use std::collections::HashMap;
use std::convert::From;
use std::convert::TryInto;
use std::sync::Arc;

use anyhow::Error;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use std::fmt::Debug;
use crate::version::v0_0_1::bin::Bin;


pub type State=HashMap<String,Bin>;

pub type ArtifactRef=String;
pub type Artifact=Arc<Vec<u8>>;
pub type Port=String;

pub mod id {
    use crate::version::v0_0_1::generic;
    use serde::{Serialize,Deserialize};
    use std::str::FromStr;
    use anyhow::Error;
    use crate::version::v0_0_1::parse::{parse_specific, parse_address, parse_version};

    pub type Key = String;
    pub type Kind = String;
    pub type ResourceType = String;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum IdentifierKind {
        Key,
        Address
    }

    pub type Identifier = generic::id::Identifier<Key,Address>;
    pub type Identifiers = generic::id::Identifiers<Key,Address>;


    #[derive(Debug,Clone,Serialize,Deserialize,Eq,PartialEq,Hash)]
    pub struct Address {
        segments: Vec<String>
    }

    impl Address {
        pub fn parent(&self) -> Option<Address> {
            if self.segments.is_empty() {
                return Option::None;
            }
            let mut segments = self.segments.clone();
            segments.remove( segments.len() );
            Option::Some( Self {
                segments
            })
        }
    }

    impl FromStr for Address {
        type Err = Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let (leftover,segments) = parse_address(s)?;
            if leftover.len() != 0 {
                Err(anyhow!(format!("could not parse entire address: '{}' leftover '{}'", s, leftover )))
            } else {
                Ok(Self{
                    segments
                })
            }
        }
    }

    impl ToString for Address {
        fn to_string(&self) -> String {
            let mut rtn = String::new();
            for (i, segment) in self.segments.iter().enumerate() {
                rtn.push_str( segment.as_str() );
                if i != self.segments.len() {
                    rtn.push_str(":");
                }
            }
            rtn.to_string()
        }
    }


    #[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
    pub struct Specific {
        pub vendor: String,
        pub product: String,
        pub variant: String,
        pub version: Version,
    }

    impl ToString for Specific {
        fn to_string(&self) -> String {
            format!(
                "{}:{}:{}:{}",
                self.vendor.to_string(),
                self.product,
                self.variant,
                self.version.to_string()
            )
        }
    }

    impl FromStr for Specific {
        type Err = Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let (leftover, specific) = parse_specific(s)?;
            if leftover.len() != 0 {
                let message = format!(
                    "could not process '{}' portion of Specific '{}'",
                    leftover, s
                );
                Err(anyhow!(message))
            } else {
                Ok(specific)
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
    pub struct Version {
        major: usize,
        minor: usize,
        patch: usize,
        release: Option<String>,
    }

    impl Version {
        pub fn new(major: usize, minor: usize, patch: usize, release: Option<String>) -> Self {
            Self {
                major,
                minor,
                patch,
                release,
            }
        }
    }


    impl ToString for Version {
        fn to_string(&self) -> String {
            match &self.release {
                None => {
                    format!("{}.{}.{}", self.major, self.minor, self.patch)
                }
                Some(release) => {
                    format!(
                        "{}.{}.{}-{}",
                        self.major,
                        self.minor,
                        self.patch,
                        release.to_string()
                    )
                }
            }
        }
    }

    impl FromStr for Version {
        type Err = Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let string = s.to_string();
            let s: &'static str = string.as_str();
            let (remaining, version) = parse_version(s)?;
            if remaining.len() > 0 {
                Err(anyhow!(format!(
                    "could not parse '{}' portion for version string '{}",
                    remaining, s
                )
                    ))
            } else {
                Ok(version)
            }
        }
    }

}

pub mod messaging {
    use serde::{Serialize,Deserialize};
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
    use serde::{Serialize,Deserialize};
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
    use serde::{Serialize,Deserialize};
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
    use serde::{Serialize,Deserialize};
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
    use serde::{Serialize,Deserialize};
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
    pub type ResponseEntity = generic::delivery::ResponseEntity<Key,Address,Kind>;
}

pub mod command {

    use serde::{Serialize,Deserialize};

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

    use serde::{Serialize,Deserialize};
    use crate::version::v0_0_1::id::{Key, Address, Kind};

    use crate::version::v0_0_1::generic;
    use std::collections::HashMap;
    use crate::version::latest::ArtifactRef;

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

    use crate::version::v0_0_1::{State, http};
    use crate::version::v0_0_1::generic;
    use crate::version::v0_0_1::generic::operation;

    pub type Operation = operation::Operation<Key,Address,Kind>;
    pub type ResourceOperation = operation::ResourceOperation<Key,Address,Kind>;
    pub type ExtOperation = operation::ExtOperation<Key,Address,Kind>;
    pub type PortOperation = operation::PortOperation<Key,Address,Kind>;
}

pub mod resource {
    use crate::version::v0_0_1::id::{Key, Address, Kind};
    use serde::{Deserialize, Serialize};
    use crate::version::v0_0_1::generic;

    #[derive(Debug, Clone, Serialize, Deserialize, strum_macros::Display)]
    pub enum Status {
        Unknown,
        Initializing,
        Ready,
        Panic(String),
        Done
    }


    pub type Create=generic::resource::Create<Key,Address,Kind>;

    pub type StateSrc=generic::resource::StateSrc;
    pub type CreateStrategy=generic::resource::CreateStrategy;
    pub type AddressSrc=generic::resource::AddressSrc;
    pub type Selector=generic::resource::Selector;
    pub type MetaSelector=generic::resource::MetaSelector;
    pub type ResourceStub = generic::resource::ResourceStub<Key,Address,Kind>;
}

pub mod portal {
    pub mod inlet {
        use crate::version::v0_0_1::id::{Key, Address, Kind};
        use std::convert::TryFrom;
        use std::convert::TryInto;

        use anyhow::Error;
        use serde::{Deserialize, Serialize};
        use crate::version::v0_0_1::generic;


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
        use crate::version::v0_0_1::generic;

        pub type Request=generic::portal::outlet::Request<Key,Address,Kind>;
        pub type Response=generic::portal::outlet::Response<Key,Address,Kind>;
        pub type Frame=generic::portal::outlet::Frame<Key,Address,Kind>;

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
    use std::str::FromStr;

    //pub trait Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync {}

    pub mod id {
        use serde::{Serialize,Deserialize};
        use crate::version::v0_0_1::generic;
        use std::fmt::Debug;
        use std::hash::Hash;
        use std::str::FromStr;

        #[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
        pub enum Identifier<KEY: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, ADDRESS: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync> {
            Key(KEY),
            Address(ADDRESS)
        }

        #[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
        pub struct Identifiers<KEY: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, ADDRESS: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync> {
            pub key: KEY,
            pub address: ADDRESS
        }
    }




    pub mod config {
        use serde::{Serialize,Deserialize};
        use crate::version::v0_0_1::config::{PortalKind, Config};
        use crate::version::v0_0_1::ArtifactRef;
        use std::fmt::Debug;
        use std::hash::Hash;
        use std::str::FromStr;
        use crate::version::v0_0_1::generic::id::{Identifier, Identifiers};
        use crate::version::v0_0_1::generic::resource::Archetype;


        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct Info<KEY: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, ADDRESS: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, KIND: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync> {
            pub key: KEY,
            pub address: ADDRESS,
            pub owner: String,
            pub parent: Identifier<KEY,ADDRESS>,
            pub archetype: Archetype<KIND>,
            pub config: Config,
            pub ext_config: Option<ArtifactRef>,
            pub kind: PortalKind
        }

        impl <KEY: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, ADDRESS: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, KIND: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync> Info<KEY, ADDRESS, KIND> {
            pub fn identity(&self) -> Identifiers<KEY,ADDRESS> {
                Identifiers {
                    key: self.key.clone(),
                    address: self.address.clone()
                }
            }
        }
    }

    pub mod operation {
        use serde::{Serialize,Deserialize};
        use crate::version::v0_0_1::generic;
        use crate::version::latest::{State, http};
        use serde::__private::fmt::Debug;
        use std::hash::Hash;
        use std::str::FromStr;
        use crate::version::v0_0_1::generic::resource::{Create, Selector};
        use crate::version::v0_0_1::generic::delivery::Entity;

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub enum Operation<KEY: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, ADDRESS: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, KIND: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync>  {
            Resource(ResourceOperation<KEY,ADDRESS,KIND>),
            Ext(ExtOperation<KEY,ADDRESS,KIND>)
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub enum ResourceOperation<KEY: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, ADDRESS: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, KIND: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync>  {
            Create(Create<KEY,ADDRESS,KIND>),
            Select(Selector),
            Get,
            Set(State),
            Delete,
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub enum ExtOperation<KEY: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, ADDRESS: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, KIND: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync> {
            Http(http::HttpRequest),
            Port(PortOperation<KEY,ADDRESS,KIND>)
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct PortOperation<KEY: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, ADDRESS: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, KIND: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync> {
            pub port: String,
            pub entity: Entity<KEY,ADDRESS,KIND>
        }
    }


    pub mod resource {
        use serde::{Deserialize, Serialize};
        use crate::version::v0_0_1::State;
        use crate::version::v0_0_1::generic;
        use std::fmt::Debug;
        use std::hash::Hash;
        use std::str::FromStr;
        use crate::version::v0_0_1::generic::id::Identifier;

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct Archetype<KIND: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync> {
            pub kind: KIND,
            pub specific: Option<String>,
            pub config_src: Option<String>
        }


        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct ResourceStub<KEY: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, ADDRESS: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, KIND: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync> {
            pub id: Identifier<KEY,ADDRESS>,
            pub key: KEY,
            pub address: ADDRESS,
            pub archetype: Archetype<KIND>
        }



        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct Create<KEY: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, ADDRESS: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, KIND: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync> {
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
            use crate::version::v0_0_1::messaging::ExchangeKind;
            use crate::version::v0_0_1::messaging::ExchangeId;
            use crate::version::v0_0_1::log::Log;
            use crate::version::v0_0_1::command::Command;
            use crate::version::v0_0_1::resource::Status;
            use crate::version::v0_0_1::bin::BinParcel;
            use crate::version::v0_0_1::frame::{CloseReason, PrimitiveFrame};
            use crate::version::v0_0_1::id::{Address, Kind, Key};
            use std::fmt::Debug;
            use crate::version::v0_0_1::generic::id::Identifier;
            use crate::version::v0_0_1::generic::operation::Operation;
            use std::hash::Hash;
            use std::str::FromStr;
            use crate::version::v0_0_1::generic::delivery::ResponseEntity;


            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub struct Request<KEY: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, ADDRESS: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, KIND: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync>  {
                pub to: Vec<Identifier<KEY,ADDRESS>>,
                pub operation: Operation<KEY,ADDRESS,KIND>,
                pub kind: ExchangeKind,
            }

            impl <KEY: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, ADDRESS: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, KIND: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync> Request<KEY,ADDRESS,KIND> {
                pub fn new(operation: Operation<KEY,ADDRESS,KIND>) -> Self {
                    Self {
                        to: vec![],
                        operation,
                        kind: ExchangeKind::None
                    }
                }
            }

            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub struct Response<KEY: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, ADDRESS: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, KIND: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync> {
                pub to: Identifier<KEY,ADDRESS>,
                pub exchange_id: ExchangeId,
                pub signal: ResponseEntity<KEY,ADDRESS,KIND>,
            }

            #[derive(Debug, Clone, Serialize, Deserialize, strum_macros::Display)]
            pub enum Frame<KEY: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, ADDRESS: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, KIND: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync> {
                Log(Log),
                Command(Command),
                Request(Request<KEY,ADDRESS,KIND>),
                Response(Response<KEY,ADDRESS,KIND>),
                Status(Status),
                BinParcel(BinParcel),
                Close(CloseReason)
            }

            impl <KEY: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, ADDRESS: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, KIND: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync> TryInto<PrimitiveFrame> for Frame<KEY,ADDRESS,KIND> {
                type Error = Error;

                fn try_into(self) -> Result<PrimitiveFrame, Self::Error> {
                    let data = bincode::serialize(&self)?;
                    Ok(PrimitiveFrame {
                        data
                    })
                }
            }

            impl TryFrom<PrimitiveFrame> for Frame<Key,Address,Kind>{
                type Error = Error;

                fn try_from(value: PrimitiveFrame) -> Result<Self, Self::Error> {
                    let frame = bincode::deserialize(value.data.as_slice())?;
                    Ok(frame)
                }
            }

            /*
            impl <KEY: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, ADDRESS: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, KIND: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync> TryFrom<PrimitiveFrame> for Frame<KEY,ADDRESS,KIND>{
                type Error = Error;

                fn try_from(value: PrimitiveFrame) -> Result<Self, Self::Error> {
                    let frame = bincode::deserialize(value.data.as_slice())?;
                    Ok(frame)
                }
            }

             */
        }

        pub mod outlet {
            use std::convert::TryFrom;
            use std::convert::TryInto;

            use anyhow::Error;
            use serde::{Deserialize, Serialize};
            use crate::version::v0_0_1::messaging::{ExchangeKind, ExchangeId};
            use crate::version::v0_0_1::generic;
            use crate::version::v0_0_1::command::CommandEvent;
            use crate::version::v0_0_1::bin::BinParcel;
            use crate::version::v0_0_1::frame::{CloseReason, PrimitiveFrame};
            use std::fmt::Debug;
            use std::hash::Hash;
            use std::str::FromStr;
            use crate::version::v0_0_1::generic::id::Identifier;
            use crate::version::v0_0_1::generic::operation::ExtOperation;
            use crate::version::v0_0_1::generic::delivery::ResponseEntity;
            use crate::version::v0_0_1::generic::config::Info;
            use crate::version::v0_0_1::id::{Key, Address, Kind};

            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub struct Request<KEY: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, ADDRESS: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, KIND: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync> {
                pub from: Identifier<KEY,ADDRESS>,
                pub operation: ExtOperation<KEY,ADDRESS,KIND>,
                pub kind: ExchangeKind
            }

            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub struct Response<KEY: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, ADDRESS: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, KIND: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync> {
                pub from: Identifier<KEY,ADDRESS>,
                pub exchange_id: ExchangeId,
                pub signal: ResponseEntity<KEY,ADDRESS,KIND>,
            }

            #[derive(Debug, Clone, Serialize, Deserialize, strum_macros::Display)]
            pub enum Frame<KEY: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, ADDRESS: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, KIND: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync> {
                Init(Info<KEY,ADDRESS,KIND>),
                CommandEvent(CommandEvent),
                Request(Request<KEY,ADDRESS,KIND>),
                Response(Response<KEY,ADDRESS,KIND>),
                BinParcel(BinParcel),
                Close(CloseReason)
            }

            impl <KEY: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, ADDRESS: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, KIND: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync> TryInto<PrimitiveFrame> for Frame<KEY,ADDRESS,KIND> {
                type Error = Error;

                fn try_into(self) -> Result<PrimitiveFrame, Self::Error> {
                    let data = bincode::serialize(&self)?;
                    Ok(PrimitiveFrame {
                        data
                    })
                }
            }


            impl TryFrom<PrimitiveFrame> for Frame<Key,Address,Kind> {
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

        use crate::version::v0_0_1::{State, http};
        use crate::version::v0_0_1::delivery::Payload;
        use crate::version::v0_0_1::generic;
        use std::fmt::Debug;
        use std::hash::Hash;
        use std::str::FromStr;
        use crate::version::v0_0_1::generic::resource::ResourceStub;

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub enum Entity<KEY: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, ADDRESS: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, KIND: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync> {
            Empty,
            Resource(ResourceEntity<KEY,ADDRESS,KIND>),
            Payload(Payload),
            HttpResponse(http::HttpResponse)
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub enum ResourceEntity<KEY: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, ADDRESS: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, KIND: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync> {
            None,
            Stub(ResourceStub<KEY,ADDRESS,KIND>),
            Stubs(Vec<ResourceStub<KEY,ADDRESS,KIND>>),
            State(State)
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub enum ResponseEntity<KEY: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, ADDRESS: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync, KIND: Debug + Clone + Serialize + Eq + PartialEq + Hash + ToString + FromStr + Send + Sync> {
            Ok(Entity<KEY,ADDRESS,KIND>),
            Error(String)
        }

    }

    mod fail {
        use serde::{Deserialize, Serialize};
        use crate::version::v0_0_1::id::Specific;


        mod mesh {
            use serde::{Deserialize, Serialize};
            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub enum Fail{
                Error(String),
                QueueOverflow
            }
        }

        mod mechtron {
            use serde::{Deserialize, Serialize};
            use crate::version::v0_0_1::generic::fail::{resource, port, http};

            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub enum Fail{
                Error(String),
                Resource(resource::Fail),
                Port(port::Fail),
                Http(http::Error),
            }
        }

        mod resource {
            use serde::{Deserialize, Serialize};
            use crate::version::v0_0_1::id::Address;
            use crate::version::v0_0_1::generic::fail::{NotFound, Bad, Standard};

            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub enum Fail {
                Create(Create),
                Update(Update),
                Standard(Standard)
        }

            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub enum Create{
                AddressAlreadyInUse(String),
            }

            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub enum Update{
                Immutable
            }
        }




        mod port {
            use serde::{Deserialize, Serialize};
            use crate::version::v0_0_1::generic::fail::Standard;

            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub enum Fail {
                Error(String),
                Standard(Standard)
            }
        }

        mod http {
            use serde::{Deserialize, Serialize};
            use crate::version::v0_0_1::generic::fail::Standard;

            #[derive(Debug, Clone, Serialize, Deserialize)]
            pub struct Error{
                pub code: u32,
                pub message: String
            }
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub enum Standard{
           NotFound(NotFound),
           Bad(Bad),
           Illegal(Illegal),
           Wrong(Wrong),
           Timeout(Timeout)
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct Timeout {
            pub waited: i32,
            pub message: String
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub enum NotFound {
            ResourceType(String),
            Kind(String),
            Specific(String),
            Address(String),
            Key(String),
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub enum Bad {
            ResourceType(String),
            Kind(String),
            Specific(String),
            Address(String),
            Key(String),
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub enum Identifier {
            ResourceType,
            Kind,
            Specific,
            Address,
            Key,
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub enum Illegal{
           Immutable
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct Wrong {
            pub received: String,
            pub expected: String,
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub enum Fail {
            Mesh(mesh::Fail),
            Resource(resource::Fail),
            Mechtron(mechtron::Fail),
        }
    }
}

