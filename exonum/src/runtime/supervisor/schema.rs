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

use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use exonum_merkledb::{BinaryValue, IndexAccess, ObjectHash, ProofMapIndex};

use std::{
    borrow::Cow,
    collections::BTreeSet,
    io::{Cursor, Write},
};

use super::{DeployArtifact, StartService};
use crate::crypto::{self, Hash, PublicKey};

/// Service information schema.
#[derive(Debug)]
pub struct Schema<'a, T> {
    access: T,
    instance_name: &'a str,
}

impl<'a, T: IndexAccess> Schema<'a, T> {
    /// Constructs schema for the given `access`.
    pub fn new(instance_name: &'a str, access: T) -> Self {
        Self {
            instance_name,
            access,
        }
    }

    pub fn pending_artifacts(&self) -> ProofMapIndex<T, DeployArtifact, BinarySet<PublicKey>> {
        ProofMapIndex::new(
            [self.instance_name, ".pending_artifacts"].concat(),
            self.access.clone(),
        )
    }

    pub fn pending_instances(&self) -> ProofMapIndex<T, StartService, BinarySet<PublicKey>> {
        ProofMapIndex::new(
            [self.instance_name, ".pending_instances"].concat(),
            self.access.clone(),
        )
    }

    pub fn confirm_pending_artifact(&mut self, id: &DeployArtifact, author: PublicKey) -> usize {
        let mut pending_artifacts = self.pending_artifacts();
        let mut confirmations = pending_artifacts.get(&id).unwrap_or_default();
        confirmations.0.insert(author);
        let len = confirmations.0.len();
        pending_artifacts.put(&id, confirmations);
        len
    }

    pub fn confirm_pending_instance(
        &mut self,
        instance_spec: &StartService,
        author: PublicKey,
    ) -> usize {
        let mut pending_instances = self.pending_instances();
        let mut confirmations = pending_instances.get(&instance_spec).unwrap_or_default();
        confirmations.0.insert(author);
        let len = confirmations.0.len();
        pending_instances.put(&instance_spec, confirmations);
        len
    }

    /// Returns hashes for tables with proofs.
    pub fn state_hash(&self) -> Vec<Hash> {
        vec![
            self.pending_artifacts().object_hash(),
            self.pending_instances().object_hash(),
        ]
    }
}

/// A set of binary values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Eq, PartialOrd, Ord)]
pub struct BinarySet<T: Ord>(pub BTreeSet<T>);

impl<T: Ord> BinarySet<T> {
    pub fn new() -> Self {
        Self(BTreeSet::default())
    }
}

impl<T: Ord> Default for BinarySet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Ord + BinaryValue> BinaryValue for BinarySet<T> {
    fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Cursor::new(Vec::new());
        for value in &self.0 {
            let bytes = value.to_bytes();
            buf.write_u64::<LittleEndian>(bytes.len() as u64).unwrap();
            buf.write_all(&bytes).unwrap();
        }
        buf.into_inner()
    }

    fn from_bytes(value: Cow<[u8]>) -> Result<Self, failure::Error> {
        let mut values = BTreeSet::new();

        let mut reader = value.as_ref();
        while !reader.is_empty() {
            let bytes_len = LittleEndian::read_u64(reader) as usize;
            reader = &reader[8..];
            let value = T::from_bytes(Cow::Borrowed(&reader[0..bytes_len]))?;
            reader = &reader[bytes_len..];
            values.insert(value);
        }

        Ok(Self(values))
    }
}

impl<T: Ord + BinaryValue> ObjectHash for BinarySet<T> {
    fn object_hash(&self) -> Hash {
        crypto::hash(&self.to_bytes())
    }
}

#[test]
fn test_validator_values_binary_value() {
    let _ = crate::helpers::init_logger();

    let mut set = BinarySet::default();
    let data = vec![
        b"abacaba1224634abcfdfdfca353".to_vec(),
        b"abacaba1224634abcfdfdfca353ee2224774".to_vec(),
    ];
    set.0.insert(data[1].clone());
    set.0.insert(data[0].clone());
    assert_eq!(set.0.len(), 2);

    let bytes = set.clone().into_bytes();
    let set2 = BinarySet::from_bytes(bytes.into()).unwrap();
    assert_eq!(set, set2);
}