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

// TODO: Replace with explicit macro import
#[macro_use]
extern crate serde_derive;

// HACK: Silent "dead_code" warning.
pub use crate::inc::{IncService, TxInc, SERVICE_ID, SERVICE_NAME};

use serde_json::json;

// use exonum::{helpers::Height, runtime::rust::Transaction};
// use exonum_merkledb::{BinaryValue, ObjectHash};
use exonum::{
    api::{self, node::public::system::DispatcherInfo},
    crypto,
    runtime::rust::Transaction,
};
use exonum_testkit::{ApiKind, InstanceCollection, TestKitApi, TestKitBuilder};

// TODO: merge lines
use exonum::api::node::public::explorer::TransactionQuery;
use exonum::crypto::Hash;
use exonum::helpers::Height;
use exonum::proto::Any;
use exonum::runtime::supervisor::{DeployRequest, StartService};
use exonum::runtime::ArtifactId;

//use exonum::runtime::supervisor::PrivateApi;

mod inc;
mod proto;

/*
// TODO: remove lifetime
impl<'a> PrivateApi for TestKitApi {
    // TODO: err (в анкоринге или еще где в сторонних серивисах)

    // TODO: ...
    fn deploy_artifact(&self, artifact: DeployRequest) -> Result<Hash, Self::Error> {
        // dbg!("ApiImpl::deploy_artifact");
        // self.broadcast_transaction(artifact).map_err(From::from)
        // ...


        // TODO: TODO: TODO: tobytes, tohex
    }

    fn start_service(&self, service: StartService) -> Result<Hash, Self::Error> {
        // self.broadcast_transaction(service).map_err(From::from)
    }
}
*/

//fn deploy_artifact(artifact: DeployRequest) -> Result<Hash, Self::Error> {
//    // ???
//}

fn inc_service_instances(factory: IncService) -> InstanceCollection {
    InstanceCollection::new(factory).with_instance(SERVICE_ID, SERVICE_NAME, ())
}

fn assert_count(api: &TestKitApi, service_name: &'static str, expected_count: u64) {
    let real_count: u64 = api
        // .public(ApiKind::Service(SERVICE_NAME))
        .public(ApiKind::Service(service_name))
        .get("v1/counter")
        .unwrap();
    assert_eq!(real_count, expected_count);
}

fn assert_no_count(api: &TestKitApi) {
    let response: api::Result<u64> = api.public(ApiKind::Service(SERVICE_NAME)).get("v1/counter");
    assert!(response.is_err());
}

#[test]
fn test_static_service() {
    let (key_pub, key_priv) = crypto::gen_keypair();
    let service = IncService::new();
    let mut testkit = TestKitBuilder::validator()
        .with_service(inc_service_instances(service.clone()))
        // .with_service(InstanceCollection::new(service.clone()))
        .create();
    let api = testkit.api();

    assert_no_count(&api);

    api.send(TxInc { seed: 0 }.sign(SERVICE_ID, key_pub, &key_priv));
    testkit.create_block();
    assert_count(&api, SERVICE_NAME, 1);

    api.send(TxInc { seed: 1 }.sign(SERVICE_ID, key_pub, &key_priv));
    testkit.create_block();
    assert_count(&api, SERVICE_NAME, 2);
}

#[test]
fn test_dynamic_service_basics() {
    let service = IncService::new();
    let mut testkit = TestKitBuilder::validator()
        .with_service(InstanceCollection::new(service.clone()))
        .create();
    let api = testkit.api();

    let info: DispatcherInfo = api.public(ApiKind::System).get("v1/services").unwrap();
    /*let expected = HealthCheckInfo {
        consensus_status: ConsensusStatus::Enabled,
        connected_peers: 0,
    };
    assert_eq!(info, expected);*/
    assert_eq!(info.artifacts.len(), 1);
    assert_eq!(info.services.len(), 1);
    println!("TEST: v1/services: {:#?}", info);

    let runtime_id_rust = 0;

    // TODO: extract a helper function
    let request = DeployRequest {
        artifact: ArtifactId {
            runtime_id: runtime_id_rust,
            name: "inc/1.0.0".into(),
        },
        spec: Any::default(),
        deadline_height: Height(10_000_000),
    };
    let result: api::Result<crypto::Hash> = api
        .private(ApiKind::Service("supervisor"))
        .query(&request)
        .post("deploy-artifact");
    dbg!(&result);
    testkit.create_block();

    // TODO: Not completely sure why I need this :(
    std::thread::sleep(std::time::Duration::from_millis(1000));

    let api = testkit.api(); // update the API

    // TODO: Move unwrap to the main call (above)
    let hash = result.unwrap();
    assert_tx_status(&api, hash, &json!({ "type": "success" }));

    /*
    {
        let info: serde_json::Value = api
            .public(ApiKind::Explorer)
            .query(&TransactionQuery::new(hash))
            .get("v1/transactions")
            .unwrap();
        dbg!(&info);
    }
    */

    let info: DispatcherInfo = api.public(ApiKind::System).get("v1/services").unwrap();
    println!("TEST: v1/services: {:#?}", info);

    let instance_name = "ozkriff_test";

    // TODO: extract a helper function
    let request = StartService {
        artifact: ArtifactId {
            runtime_id: runtime_id_rust,
            name: "inc/1.0.0".into(),
        },
        name: instance_name.into(),
        config: Any::default(),
        deadline_height: Height(10_000_000),
    };
    let result: api::Result<crypto::Hash> = api
        .private(ApiKind::Service("supervisor"))
        .query(&request)
        .post("start-service");
    dbg!(&result);
    testkit.create_block();

    let hash = result.unwrap();
    assert_tx_status(&api, hash, &json!({ "type": "success" }));

    // TODO: Not completely sure why I need this :(
    std::thread::sleep(std::time::Duration::from_millis(1000));

    let api = testkit.api(); // Update the API

    let info: DispatcherInfo = api.public(ApiKind::System).get("v1/services").unwrap();
    assert_eq!(info.services.len(), 2);
    println!("TEST: v1/services: {:#?}", info);

    // TODO: extract a helper function
    let instance_id = info.services.iter().find(|service| service.name == instance_name).unwrap().id;
    dbg!(instance_id);

    assert_no_count(&api);

    let (key_pub, key_priv) = crypto::gen_keypair();

    api.send(TxInc { seed: 0 }.sign(instance_id, key_pub, &key_priv));
    testkit.create_block();
    assert_count(&api, instance_name, 1);

    api.send(TxInc { seed: 1 }.sign(instance_id, key_pub, &key_priv));
    testkit.create_block();
    assert_count(&api, instance_name, 2);
}

#[test]
fn test_bad_deadline_height() {
    // TODO: implement

    let service = IncService::new();
    let mut testkit = TestKitBuilder::validator()
        .with_service(InstanceCollection::new(service.clone()))
        .create();
    let api = testkit.api();

    let info: DispatcherInfo = api.public(ApiKind::System).get("v1/services").unwrap();
    /*let expected = HealthCheckInfo {
        consensus_status: ConsensusStatus::Enabled,
        connected_peers: 0,
    };
    assert_eq!(info, expected);*/
    println!("TEST: v1/services: {:#?}", info);

    let runtime_id_rust = 0;

    // TODO: extract a helper function
    let request = DeployRequest {
        artifact: ArtifactId {
            runtime_id: runtime_id_rust,
            name: "inc/1.0.0".into(),
        },
        spec: Any::default(),
        deadline_height: Height(0), // TODO: BAD HEIGHT HERE
    };
    let result: api::Result<crypto::Hash> = api
        .private(ApiKind::Service("supervisor"))
        .query(&request)
        .post("deploy-artifact");
    dbg!(&result);
    testkit.create_block();

    let api = testkit.api(); // update the API

    let hash = result.unwrap();
    // TODO: This should fail, isn't it???
    assert_tx_status(&api, hash, &json!({ "type": "success" })); // TODO

    // TODO: extract a helper function
    {
        let info: serde_json::Value = api
            .public(ApiKind::Explorer)
            .query(&TransactionQuery::new(hash))
            .get("v1/transactions")
            .unwrap();
        dbg!(&info);
    }

    let info: DispatcherInfo = api.public(ApiKind::System).get("v1/services").unwrap();
    println!("TEST: v1/services: {:#?}", info);

    let instance_name = "ozkriff_test";

    // TODO: extract a helper function
    let request = StartService {
        artifact: ArtifactId {
            runtime_id: runtime_id_rust,
            name: "inc/1.0.0".into(),
        },
        name: instance_name.into(),
        config: Any::default(),
        deadline_height: Height(0), // BAD HEIGHT HERE
    };
    let result: api::Result<crypto::Hash> = api
        .private(ApiKind::Service("supervisor"))
        .query(&request)
        .post("start-service");
    dbg!(&result);
    testkit.create_block();

    // TODO: Not completely sure why I need this :(
    std::thread::sleep(std::time::Duration::from_millis(1000));

    let hash = result.unwrap();
    // At lest this fails in a nice way.
    // TODO: Check the error
    assert_tx_status(&api, hash, &json!({
        "type": "service_error",
        "code": 2,
        "description": "Deadline exceeded for the current transaction.",
     })); // TODO
}

#[test]
fn test_bad_instance_name() {
    // TODO: already running instance

}

#[test]
fn test_try_run_urigisterent_service_instance() {
    // TODO: implement


    let service = IncService::new();
    let mut testkit = TestKitBuilder::validator()
        .with_service(InstanceCollection::new(service.clone()))
        .create();
    let api = testkit.api();

    let info: DispatcherInfo = api.public(ApiKind::System).get("v1/services").unwrap();
    /*let expected = HealthCheckInfo {
        consensus_status: ConsensusStatus::Enabled,
        connected_peers: 0,
    };
    assert_eq!(info, expected);*/
    println!("TEST: v1/services: {:#?}", info);

    let runtime_id_rust = 0;

    /*
    // TODO: extract a helper function
    let request = DeployRequest {
        artifact: ArtifactId {
            runtime_id: runtime_id_rust,
            name: "inc/1.0.0".into(),
        },
        spec: Any::default(),
        deadline_height: Height(0), // TODO: BAD HEIGHT HERE
    };
    let result: api::Result<crypto::Hash> = api
        .private(ApiKind::Service("supervisor"))
        .query(&request)
        .post("deploy-artifact");
    dbg!(&result);
    testkit.create_block();

    let api = testkit.api(); // update the API

    let hash = result.unwrap();
    // TODO: This should fail, isn't it???
    assert_tx_status(&api, hash, &json!({ "type": "success" })); // TODO

    // TODO: extract a helper function
    {
        let info: serde_json::Value = api
            .public(ApiKind::Explorer)
            .query(&TransactionQuery::new(hash))
            .get("v1/transactions")
            .unwrap();
        dbg!(&info);
    }
    */

    let info: DispatcherInfo = api.public(ApiKind::System).get("v1/services").unwrap();
    println!("TEST: v1/services: {:#?}", info);

    let instance_name = "ozkriff_test";

    // TODO: extract a helper function
    let request = StartService {
        artifact: ArtifactId {
            runtime_id: runtime_id_rust,
            name: "inc/1.0.0".into(),
        },
        name: instance_name.into(),
        config: Any::default(),
        deadline_height: Height(1000),
    };
    let result: api::Result<crypto::Hash> = api
        .private(ApiKind::Service("supervisor"))
        .query(&request)
        .post("start-service");
    dbg!(&result);
    testkit.create_block();

    // TODO: Not completely sure why I need this :(
    std::thread::sleep(std::time::Duration::from_millis(1000));

    let hash = result.unwrap();
    // At lest this fails in a nice way.
    // TODO: Check the error
    assert_tx_status(&api, hash, &json!({
        "type": "service_error",
        "code": 2,
        "description": "Deadline exceeded for the current transaction.",
     })); // TODO
}

#[test]
fn test_bad_service_name() {
    // TODO: implement

    let service = IncService::new();
    let mut testkit = TestKitBuilder::validator()
        .with_service(InstanceCollection::new(service.clone()))
        .create();
    let api = testkit.api();

    let info: DispatcherInfo = api.public(ApiKind::System).get("v1/services").unwrap();
    println!("TEST: v1/services: {:#?}", info);

    let runtime_id_rust = 0;

    // TODO: extract a helper function
    let request = DeployRequest {
        artifact: ArtifactId {
            runtime_id: runtime_id_rust,
            name: "doesnotexist/1.0.0".into(),
        },
        spec: Any::default(),
        deadline_height: Height(100000), // TODO: BAD HEIGHT HERE
    };
    let result: api::Result<crypto::Hash> = api
        .private(ApiKind::Service("supervisor"))
        .query(&request)
        .post("deploy-artifact");
    dbg!(&result);
    testkit.create_block();

    let api = testkit.api(); // update the API

    // TODO: Not completely sure why I need this :(
    std::thread::sleep(std::time::Duration::from_millis(1000));

    let hash = result.unwrap();
    // TODO: This should fail, isn't it???
    assert_tx_status(&api, hash, &json!({ "type": "success" })); // TODO

    let info: DispatcherInfo = api.public(ApiKind::System).get("v1/services").unwrap();
    println!("TEST: v1/services: {:#?}", info);

    panic!("ozkriff: expected"); // TODO: remove when done

}

// TODO: Extract this function somewhere?
fn assert_tx_status(api: &TestKitApi, tx_hash: Hash, expected_status: &serde_json::Value) {
    let info: serde_json::Value = api
        .public(ApiKind::Explorer)
        .query(&TransactionQuery::new(tx_hash))
        .get("v1/transactions")
        .unwrap();

    if let serde_json::Value::Object(mut info) = info {
        let tx_status = info.remove("status").unwrap();
        assert_eq!(tx_status, *expected_status);
    } else {
        panic!("Invalid transaction info format, object expected");
    }
}
