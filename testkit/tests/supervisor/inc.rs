// Copyright 2019 The Exonum Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::proto;
use exonum::{
    api,
    blockchain::ExecutionError,
    runtime::{
        rust::{RustArtifactId, Service, ServiceDescriptor, ServiceFactory, TransactionContext},
        ArtifactInfo, ServiceInstanceId,
    },
};
use exonum_derive::{exonum_service, ProtobufConvert};
use exonum_merkledb::{Entry, IndexAccess};

pub const SERVICE_ID: ServiceInstanceId = 512;
pub const SERVICE_NAME: &str = "inc";

/// Schema of the key-value storage used by the demo cryptocurrency service.
#[derive(Debug)]
pub struct Schema<'a, T> {
    name: &'a str,
    access: T,
}

impl<'a, T: IndexAccess> Schema<'a, T> {
    pub fn new(name: &'a str, access: T) -> Self {
        Schema { name, access }
    }

    fn index_name(&self, name: &str) -> String {
        [SERVICE_NAME, ".", name].concat()
    }

    fn entry(&self) -> Entry<T, u64> {
        Entry::new(self.index_name("count"), self.access.clone())
    }

    pub fn count(&self) -> Option<u64> {
        self.entry().get()
    }

    fn inc(&mut self) -> u64 {
        let count = self
            .count()
            .unwrap_or(0)
            .checked_add(1)
            .expect("attempt to add with overflow");
        self.entry().set(count);
        count
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, ProtobufConvert, PartialEq)]
#[exonum(pb = "proto::TxInc")]
pub struct TxInc {
    pub seed: u64,
}

#[exonum_service(dispatcher = "IncService")]
pub trait IncInterface {
    fn inc(&self, context: TransactionContext, arg: TxInc) -> Result<(), ExecutionError>;
}

#[derive(Clone, Default, Debug)]
pub struct IncService;

impl IncInterface for IncService {
    fn inc(&self, context: TransactionContext, _arg: TxInc) -> Result<(), ExecutionError> {
        let mut schema = Schema::new(context.service_name(), context.fork());
        schema.inc();
        Ok(())
    }
}

impl IncService {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone)]
pub struct PublicApi {
    service_name: String,
    service_id: ServiceInstanceId,
}

impl PublicApi {
    /// Creates a new public API for the specified service instance.
    pub fn new(descriptor: ServiceDescriptor) -> Self {
        Self {
            service_name: descriptor.service_name().to_owned(),
            service_id: descriptor.service_id(),
        }
    }

    fn counter(&self, state: &api::ServiceApiState) -> api::Result<u64> {
        let snapshot = state.snapshot();
        let schema = Schema::new(&self.service_name, &snapshot);
        schema
            .count()
            .ok_or_else(|| api::Error::NotFound("Counter is not set yet".to_owned()))
    }

    fn wire(self, builder: &mut api::ServiceApiBuilder) {
        builder.public_scope().endpoint(
            "v1/counter",
            move |state: &api::ServiceApiState, _query: ()| self.counter(state),
        );
    }
}

impl Service for IncService {
    fn wire_api(&self, descriptor: ServiceDescriptor, builder: &mut api::ServiceApiBuilder) {
        PublicApi::new(descriptor).wire(builder);
        // panic!("JUST TESTING") // TODO: ???
    }
}

impl ServiceFactory for IncService {
    fn artifact_id(&self) -> RustArtifactId {
        "inc/1.0.0".parse().unwrap()
    }

    fn artifact_info(&self) -> ArtifactInfo {
        ArtifactInfo {
            proto_sources: proto::PROTO_SOURCES.as_ref(),
        }
    }

    fn create_instance(&self) -> Box<dyn Service> {
        Box::new(self.clone())
    }
}
