use crate::version::v0_0_1;

pub type ExchangeId = v0_0_1::ExchangeId;
pub type State=v0_0_1::State;
pub type Key=v0_0_1::Key;
pub type Address=v0_0_1::Address;
pub type Kind=v0_0_1::Kind;
pub type ArtifactRef=v0_0_1::ArtifactRef;
pub type Artifact=v0_0_1::Artifact;
pub type Port=v0_0_1::Port;
pub type IdentifierKind=v0_0_1::IdentifierKind;

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
    use crate::version::latest::{Key, Address, Kind};
    use serde::{Deserialize, Serialize};
    use crate::version::v0_0_1::resource;

    use crate::version::v0_0_1::{State, generic};
    use crate::version::v0_0_1::generic::{ExtOperation, Generic, Identifier};
    use crate::version::v0_0_1::generic::resource::Archetype;

    pub type Status = resource::Status;

    pub type Operation=generic::resource::Operation<Key,Address,Kind>;
    pub type ResourceOperation=generic::resource::ResourceOperation<Key,Address,Kind>;
    pub type Create=generic::resource::Create<Key,Address,Kind>;

    pub type StateSrc=generic::resource::StateSrc;
    pub type CreateStrategy=generic::resource::CreateStrategy;
    pub type AddressSrc=generic::resource::AddressSrc;
    pub type Selector=generic::resource::Selector;
    pub type MetaSelector=generic::resource::MetaSelector;
}

pub mod config {
    use crate::version::latest::{Key, Address, Kind};
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
    use crate::version::latest::{Key, Address, Kind};
    use crate::version::v0_0_1::delivery;
    use crate::version::v0_0_1::generic;

    pub type Payload = delivery::Payload;
    pub type Entity = generic::delivery::Entity<Key,Address,Kind>;
    pub type ResourceEntity = generic::delivery::ResourceEntity<Key,Address,Kind>;
    pub type ResponseSignal = generic::delivery::ResponseEntity<Key,Address,Kind>;
}



pub mod operation {
    use crate::version::latest::{Key, Address, Kind};

    use crate::version::v0_0_1::{Generic, State, http};
    use crate::version::v0_0_1::generic::resource::{Create, Selector};
    use crate::version::v0_0_1::generic::operation;

    pub type Operation = operation::Operation<Key,Address,Kind>;
    pub type ResourceOperation = operation::ResourceOperation<Key,Address,Kind>;
    pub type ExtOperation = operation::ExtOperation<Key,Address,Kind>;
    pub type PortOperation = operation::PortOperation<Key,Address,Kind>;
}

pub mod portal {
    pub mod inlet {
        use crate::version::latest::{Key, Address, Kind};
        use std::convert::TryFrom;
        use std::convert::TryInto;

        use anyhow::Error;
        use serde::{Deserialize, Serialize};

        use crate::version::v0_0_1::{BinParcel, Command, ExchangeId, ExchangeKind, Identifier, Log, PrimitiveFrame, ResponseSignal, Status, CloseReason, generic};
        use crate::version::v0_0_1::generic::{Identifier, ExchangeKind, Generic, ResponseSignal, Log, Command, Status, BinParcel, CloseReason, PrimitiveFrame};

        pub type Request=generic::portal::inlet::Request<Key,Address,Kind>;
        pub type Response=generic::portal::inlet::Response<Key,Address,Kind>;
        pub type Frame=generic::portal::inlet::Frame<Key,Address,Kind>;
    }

    pub mod outlet {
        use crate::version::latest::{Key, Address, Kind};

        use std::convert::TryFrom;
        use std::convert::TryInto;

        use anyhow::Error;
        use serde::{Deserialize, Serialize};

        use crate::version::v0_0_1::{BinParcel, CliId, Entity, ExchangeId, ExchangeKind, ExtOperation, Identifier, Port, PrimitiveFrame, ResponseSignal, CloseReason, generic};
        use crate::version::v0_0_1::generic::{Identifier, ExtOperation, ExchangeKind, Generic, ResponseSignal, BinParcel, CloseReason, PrimitiveFrame};

        pub type Request=generic::portal::outlet::Request<Key,Address,Kind>;
        pub type Response=generic::portal::outlet::Response<Key,Address,Kind>;
        pub type Frame=generic::portal::outlet::Frame<Key,Address,Kind>;
        pub type CommandEvent=generic::portal::outlet::CommandEvent;
    }
}
