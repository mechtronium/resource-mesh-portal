use crate::version::v0_0_1;

pub type State=v0_0_1::State;
pub type ArtifactRef=v0_0_1::ArtifactRef;
pub type Artifact=v0_0_1::Artifact;
pub type Port=v0_0_1::Port;


pub mod id {
    use crate::version::v0_0_1::generic;

    pub type Key = String;
    pub type Address = String;
    pub type Kind = String;

    pub enum IdentifierKind {
        Key,
        Address
    }

    pub type Identifiers = generic::id::Identifiers<Key, Address>;
    pub type Identifier = generic::id::Identifier<Key, Address>;
}

pub mod messaging {
    use crate::version::v0_0_1::messaging;
    pub type ExchangeId = messaging::ExchangeId;
    pub type ExchangeKind = messaging::ExchangeKind;
}


pub mod log {
    use crate::version::v0_0_1::log;

    pub type Log = log::Log;
}



pub mod frame {
    use crate::version::v0_0_1::frame;

    pub type PrimitiveFrame = frame::PrimitiveFrame;
    pub type CloseReason = frame::CloseReason;
}

pub mod bin {
    use crate::version::v0_0_1::bin;

    pub type BinSrc = bin::BinSrc;
    pub type BinRaw = bin::BinRaw;
    pub type Bin = bin::Bin;
    pub type BinParcel = bin::BinParcel;
}

pub mod command {
    use crate::version::v0_0_1::command;

    pub type Command = command::Command;
    pub type CommandStatus = command::CommandStatus;
    pub type CliId = command::CliId;
    pub type CliEvent = command::CommandEvent;
}

pub mod http {
    use crate::version::v0_0_1::http;

    pub type HttpRequest = http::HttpRequest;
    pub type HttpResponse = http::HttpResponse;
}

pub mod resource {
    use crate::version::latest::id::{Key, Address, Kind};
    use serde::{Deserialize, Serialize};
    use crate::version::v0_0_1::resource;

    use crate::version::v0_0_1::{State, generic};

    pub type Status = resource::Status;

    pub type Operation=generic::operation::Operation<Key,Address,Kind>;
    pub type ResourceOperation=generic::operation::ResourceOperation<Key,Address,Kind>;
    pub type Create=generic::resource::Create<Key,Address,Kind>;

    pub type StateSrc=generic::resource::StateSrc;
    pub type CreateStrategy=generic::resource::CreateStrategy;
    pub type AddressSrc=generic::resource::AddressSrc;
    pub type Selector=generic::resource::Selector;
    pub type MetaSelector=generic::resource::MetaSelector;
    pub type ResourceStub = generic::resource::ResourceStub<Key,Address,Kind>;
    pub type Archetype = generic::resource::Archetype<Kind>;
}

pub mod config {
    use crate::version::latest::id::{Key, Address, Kind};
    use crate::version::v0_0_1::config;
    use crate::version::v0_0_1::generic;

    pub type Info = generic::config::Info<Key,Address,Kind>;
    pub type PortalKind = config::PortalKind;
    pub type Config = config::Config;
    pub type SchemaRef = config::SchemaRef;
    pub type BindConfig = config::BindConfig;
    pub type PortConfig = config::PortConfig;
    pub type EntityConfig = config::EntityConfig;
    pub type ResourceConfig = config::ResourceConfig;
    pub type PayloadConfig = config::PayloadConfig;
}

pub mod delivery {
    use crate::version::latest::id::{Key, Address, Kind};
    use crate::version::v0_0_1::delivery;
    use crate::version::v0_0_1::generic;

    pub type Payload = delivery::Payload;
    pub type Entity = generic::delivery::Entity<Key,Address,Kind>;
    pub type ResourceEntity = generic::delivery::ResourceEntity<Key,Address,Kind>;
    pub type ResponseEntity = generic::delivery::ResponseEntity<Key,Address,Kind>;
}



pub mod operation {
    use crate::version::latest::id::{Key, Address, Kind};

    use crate::version::v0_0_1::generic::resource::{Create, Selector};
    use crate::version::v0_0_1::generic::operation;

    pub type Operation = operation::Operation<Key,Address,Kind>;
    pub type ResourceOperation = operation::ResourceOperation<Key,Address,Kind>;
    pub type ExtOperation = operation::ExtOperation<Key,Address,Kind>;
    pub type PortOperation = operation::PortOperation<Key,Address,Kind>;
}

pub mod portal {
    pub mod inlet {
        use crate::version::latest::id::{Key, Address, Kind};
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
        use crate::version::latest::id::{Key, Address, Kind};

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
