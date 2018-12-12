// Copyright 2018 The Exonum Team
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

use rand::{thread_rng, Rng, RngCore};
use serde::Serialize;
use serde_derive::Serialize;
use serde_json::{from_str, to_string};

use self::ListProof::*;
use crate::{hash::HashTag, Database, ListProof, MemoryDB, ProofListIndex};
use exonum_crypto::{hash, Hash};

const IDX_NAME: &'static str = "idx_name";

fn random_values(len: usize) -> Vec<Vec<u8>> {
    use std::collections::HashSet;
    let mut rng = thread_rng();
    let mut exists = HashSet::new();
    let generator = |_| {
        let mut new_val: Vec<u8> = vec![0; 10];
        rng.fill_bytes(&mut new_val);

        while exists.contains(&new_val) {
            rng.fill_bytes(&mut new_val);
        }
        exists.insert(new_val.clone());
        new_val
    };

    (0..len).map(generator).collect::<Vec<_>>()
}

fn list_methods(db: Box<dyn Database>) {
    let mut fork = db.fork();
    let mut index = ProofListIndex::new(IDX_NAME, &mut fork);

    assert!(index.is_empty());
    assert_eq!(index.len(), 0);
    index.push(vec![1]);
    assert!(!index.is_empty());
    assert_eq!(index.len(), 1);

    index.push(vec![2]);
    assert_eq!(index.len(), 2);

    index.extend(vec![vec![3]]);
    assert_eq!(index.len(), 3);

    assert_eq!(index.get(0), Some(vec![1]));
    assert_eq!(index.get(1), Some(vec![2]));
    assert_eq!(index.get(2), Some(vec![3]));

    assert_eq!(index.last(), Some(vec![3]));

    index.set(1, vec![4]);
    assert_eq!(index.get(1), Some(vec![4]));

    index.clear();
    assert_eq!(index.len(), 0);
}

fn height(db: Box<dyn Database>) {
    let mut fork = db.fork();
    let mut index = ProofListIndex::new(IDX_NAME, &mut fork);

    index.push(vec![1]);
    assert_eq!(index.height(), 1);

    index.push(vec![2]);
    assert_eq!(index.height(), 2);

    index.push(vec![3]);
    assert_eq!(index.height(), 3);

    index.push(vec![4]);
    assert_eq!(index.height(), 3);

    assert_eq!(index.len(), 4);
    assert_eq!(index.get(0), Some(vec![1]));
    assert_eq!(index.get(1), Some(vec![2]));
    assert_eq!(index.get(2), Some(vec![3]));
    assert_eq!(index.get(3), Some(vec![4]));

    index.set(1, vec![10]);
    assert_eq!(index.get(1), Some(vec![10]));
}

fn iter(db: Box<dyn Database>) {
    let mut fork = db.fork();
    let mut list_index = ProofListIndex::new(IDX_NAME, &mut fork);

    list_index.extend(vec![1u8, 2, 3]);

    assert_eq!(list_index.iter().collect::<Vec<u8>>(), vec![1, 2, 3]);
    assert_eq!(list_index.iter_from(0).collect::<Vec<u8>>(), vec![1, 2, 3]);
    assert_eq!(list_index.iter_from(1).collect::<Vec<u8>>(), vec![2, 3]);
    assert_eq!(
        list_index.iter_from(3).collect::<Vec<u8>>(),
        Vec::<u8>::new()
    );
}

fn list_index_proof(db: Box<dyn Database>) {
    let mut fork = db.fork();
    let mut index = ProofListIndex::new(IDX_NAME, &mut fork);

    let h0 = HashTag::hash_leaf(2u64);
    let h1 = HashTag::hash_leaf(4u64);
    let h2 = HashTag::hash_leaf(6u64);
    let h01 = HashTag::hash_node(&h0, &h1);
    let h22 = HashTag::hash_single_node(&h2);
    let h012 = HashTag::hash_node(&h01, &h22);

    assert_eq!(index.list_hash(), HashTag::empty_list_hash());

    index.push(2u64);

    assert_eq!(index.list_hash(), HashTag::hash_list_node(1, h0));
    assert_eq!(index.get_proof(0), Leaf(2));
    assert_eq!(
        index
            .get_proof(0)
            .validate(index.list_hash(), index.len())
            .unwrap(),
        [(0, &2)]
    );

    index.push(4u64);
    assert_eq!(index.list_hash(), HashTag::hash_list_node(2, h01));
    assert_eq!(index.get_proof(0), Left(Box::new(Leaf(2)), Some(h1)));
    assert_eq!(
        index
            .get_proof(0)
            .validate(index.list_hash(), index.len())
            .unwrap(),
        [(0, &2)]
    );
    assert_eq!(index.get_proof(1), Right(h0, Box::new(Leaf(4))));
    assert_eq!(
        index
            .get_proof(1)
            .validate(index.list_hash(), index.len())
            .unwrap(),
        [(1, &4)]
    );

    assert_eq!(
        index.get_range_proof(0..2),
        Full(Box::new(Leaf(2)), Box::new(Leaf(4)))
    );
    assert_eq!(
        index
            .get_range_proof(0..2)
            .validate(index.list_hash(), index.len())
            .unwrap(),
        [(0, &2), (1, &4)]
    );

    index.push(6u64);
    assert_eq!(index.list_hash(), HashTag::hash_list_node(3, h012));
    assert_eq!(
        index.get_proof(0),
        Left(Box::new(Left(Box::new(Leaf(2)), Some(h1))), Some(h22))
    );
    assert_eq!(
        index
            .get_proof(0)
            .validate(index.list_hash(), index.len())
            .unwrap(),
        [(0, &2)]
    );
    assert_eq!(
        index.get_proof(1),
        Left(Box::new(Right(h0, Box::new(Leaf(4)))), Some(h22))
    );
    assert_eq!(
        index
            .get_proof(1)
            .validate(index.list_hash(), index.len())
            .unwrap(),
        [(1, &4)]
    );
    assert_eq!(
        index.get_proof(2),
        Right(h01, Box::new(Left(Box::new(Leaf(6)), None)))
    );
    assert_eq!(
        index
            .get_proof(2)
            .validate(index.list_hash(), index.len())
            .unwrap(),
        [(2, &6)]
    );

    assert_eq!(
        index.get_range_proof(0..2),
        Left(
            Box::new(Full(Box::new(Leaf(2)), Box::new(Leaf(4)))),
            Some(h22),
        )
    );
    assert_eq!(
        index
            .get_range_proof(0..2)
            .validate(index.list_hash(), index.len())
            .unwrap(),
        [(0, &2), (1, &4)]
    );

    assert_eq!(
        index.get_range_proof(1..3),
        Full(
            Box::new(Right(h0, Box::new(Leaf(4)))),
            Box::new(Left(Box::new(Leaf(6)), None)),
        )
    );
    assert_eq!(
        index
            .get_range_proof(1..3)
            .validate(index.list_hash(), index.len())
            .unwrap(),
        [(1, &4), (2, &6)]
    );

    assert_eq!(
        index.get_range_proof(0..3),
        Full(
            Box::new(Full(Box::new(Leaf(2)), Box::new(Leaf(4)))),
            Box::new(Left(Box::new(Leaf(6)), None)),
        )
    );
    assert_eq!(
        index
            .get_range_proof(0..3)
            .validate(index.list_hash(), index.len())
            .unwrap(),
        [(0, &2), (1, &4), (2, &6)]
    );
}

fn randomly_generate_proofs(db: Box<dyn Database>) {
    let mut fork = db.fork();
    let mut index = ProofListIndex::new(IDX_NAME, &mut fork);
    let num_values = 100;
    let values = random_values(num_values as usize);
    let mut rng = thread_rng();
    for value in &values {
        index.push(value.clone());
    }
    index.get(0);
    let table_merkle_root = index.list_hash();

    for _ in 0..50 {
        let start_range = rng.gen_range(0, num_values);
        let end_range = rng.gen_range(start_range + 1, num_values + 1);
        let range_proof = index.get_range_proof(start_range..end_range);
        {
            let (indices, actual_values): (Vec<_>, Vec<_>) = range_proof
                .validate(table_merkle_root, index.len())
                .unwrap()
                .into_iter()
                .unzip();
            assert_eq!(indices, (start_range..end_range).collect::<Vec<_>>());

            let expect_values = &values[start_range as usize..end_range as usize];
            for (expected, actual) in expect_values.iter().zip(actual_values) {
                assert_eq!(*expected, *actual);
            }
        }

        let _proof_info = ProofInfo {
            merkle_root: table_merkle_root,
            list_length: index.len(),
            proof: &range_proof,
            range_st: start_range,
            range_end: end_range,
        };

        let json_representation = to_string(&range_proof).unwrap();
        assert!(json_representation.len() > 0);
        assert_eq!(range_proof, from_str(&json_representation).unwrap());
    }
}

fn hash_leaf_node(value: &[u8]) -> Hash {
    HashTag::Leaf.hash_stream().update(value).hash()
}

fn hash_branch_node(value: &[u8]) -> Hash {
    HashTag::Node.hash_stream().update(value).hash()
}

fn index_and_proof_roots(db: Box<dyn Database>) {
    let mut fork = db.fork();
    let mut index = ProofListIndex::new(IDX_NAME, &mut fork);
    assert_eq!(index.list_hash(), HashTag::empty_list_hash());

    let h1 = hash_leaf_node(&[1, 2]);
    let h2 = hash_leaf_node(&[2, 3]);
    let h3 = hash_leaf_node(&[3, 4]);
    let h4 = hash_leaf_node(&[4, 5]);
    let h5 = hash_leaf_node(&[5, 6]);
    let h6 = hash_leaf_node(&[6, 7]);
    let h7 = hash_leaf_node(&[7, 8]);
    let h8 = hash_leaf_node(&[8, 9]);

    let h12 = hash_branch_node(&[h1.as_ref(), h2.as_ref()].concat());
    let h3up = hash_branch_node(h3.as_ref());
    let h123 = hash_branch_node(&[h12.as_ref(), h3up.as_ref()].concat());

    let h34 = hash_branch_node(&[h3.as_ref(), h4.as_ref()].concat());
    let h1234 = hash_branch_node(&[h12.as_ref(), h34.as_ref()].concat());

    let h5up = hash_branch_node(h5.as_ref());
    let h5upup = hash_branch_node(h5up.as_ref());
    let h12345 = hash_branch_node(&[h1234.as_ref(), h5upup.as_ref()].concat());

    let h56 = hash_branch_node(&[h5.as_ref(), h6.as_ref()].concat());
    let h56up = hash_branch_node(h56.as_ref());
    let h123456 = hash_branch_node(&[h1234.as_ref(), h56up.as_ref()].concat());

    let h7up = hash_branch_node(h7.as_ref());
    let h567 = hash_branch_node(&[h56.as_ref(), h7up.as_ref()].concat());
    let h1234567 = hash_branch_node(&[h1234.as_ref(), h567.as_ref()].concat());

    let h78 = hash_branch_node(&[h7.as_ref(), h8.as_ref()].concat());
    let h5678 = hash_branch_node(&[h56.as_ref(), h78.as_ref()].concat());
    let h12345678 = hash_branch_node(&[h1234.as_ref(), h5678.as_ref()].concat());

    let expected_hash_comb: Vec<(Vec<u8>, Hash, u64)> = vec![
        (vec![1, 2], HashTag::hash_list_node(1, h1), 0),
        (vec![2, 3], HashTag::hash_list_node(2, h12), 1),
        (vec![3, 4], HashTag::hash_list_node(3, h123), 2),
        (vec![4, 5], HashTag::hash_list_node(4, h1234), 3),
        (vec![5, 6], HashTag::hash_list_node(5, h12345), 4),
        (vec![6, 7], HashTag::hash_list_node(6, h123456), 5),
        (vec![7, 8], HashTag::hash_list_node(7, h1234567), 6),
        (vec![8, 9], HashTag::hash_list_node(8, h12345678), 7),
    ];

    for (inserted, exp_root, proof_ind) in expected_hash_comb {
        index.push(inserted);

        assert_eq!(index.list_hash(), exp_root);
        let range_proof = index.get_range_proof(proof_ind..proof_ind + 1);
        assert_eq!(
            range_proof
                .validate(index.list_hash(), index.len())
                .unwrap()
                .len(),
            1
        );
        let json_representation = to_string(&range_proof).unwrap();
        let deserialized_proof: ListProof<Vec<u8>> = from_str(&json_representation).unwrap();
        assert_eq!(deserialized_proof, range_proof);
        let range_proof = index.get_range_proof(0..proof_ind + 1);
        assert_eq!(
            range_proof
                .validate(index.list_hash(), index.len())
                .unwrap()
                .len(),
            (proof_ind + 1) as usize
        );
        let json_representation = to_string(&range_proof).unwrap();
        let deserialized_proof: ListProof<Vec<u8>> = from_str(&json_representation).unwrap();
        assert_eq!(deserialized_proof, range_proof);
        let range_proof = index.get_range_proof(0..1);
        assert_eq!(
            range_proof
                .validate(index.list_hash(), index.len())
                .unwrap()
                .len(),
            1
        );
        let json_representation = to_string(&range_proof).unwrap();
        let deserialized_proof: ListProof<Vec<u8>> = from_str(&json_representation).unwrap();
        assert_eq!(deserialized_proof, range_proof);
    }

    let range_proof = index.get_range_proof(0..8);
    let (indices, val_refs): (Vec<_>, Vec<_>) = range_proof
        .validate(index.list_hash(), index.len())
        .unwrap()
        .into_iter()
        .unzip();
    assert_eq!(indices, (0..8).collect::<Vec<_>>());
    let expect_values = vec![
        vec![1, 2],
        vec![2, 3],
        vec![3, 4],
        vec![4, 5],
        vec![5, 6],
        vec![6, 7],
        vec![7, 8],
        vec![8, 9],
    ];
    let paired = expect_values.into_iter().zip(val_refs);
    for pair in paired {
        assert_eq!(pair.0, *pair.1);
    }

    let mut range_proof = index.get_range_proof(3..5);
    assert_eq!(
        range_proof
            .validate(index.list_hash(), index.len())
            .unwrap()
            .len(),
        2
    );
    range_proof = index.get_range_proof(2..6);
    assert_eq!(
        range_proof
            .validate(index.list_hash(), index.len())
            .unwrap()
            .len(),
        4
    );
    assert_eq!(index.get(0), Some(vec![1, 2]));
}

fn proof_illegal_lower_bound(db: Box<dyn Database>) {
    let mut fork = db.fork();
    let mut index = ProofListIndex::new(IDX_NAME, &mut fork);
    index.get_range_proof(0..1);
    index.push(vec![1]);
}

fn proof_illegal_bound_empty(db: Box<dyn Database>) {
    let mut fork = db.fork();
    let mut index = ProofListIndex::new(IDX_NAME, &mut fork);
    for i in 0_u8..8 {
        index.push(vec![i]);
    }
    index.get_range_proof(8..9);
}

fn proof_illegal_range(db: Box<dyn Database>) {
    let mut fork = db.fork();
    let mut index = ProofListIndex::new(IDX_NAME, &mut fork);
    for i in 0_u8..4 {
        index.push(vec![i]);
    }
    index.get_range_proof(2..2);
}

fn proof_structure(db: Box<dyn Database>) {
    let mut fork = db.fork();
    let mut index = ProofListIndex::new(IDX_NAME, &mut fork);
    assert_eq!(index.list_hash(), HashTag::empty_list_hash());

    // spell-checker:ignore upup

    let h1 = hash_leaf_node(&vec![0, 1, 2]);
    let h2 = hash_leaf_node(&vec![1, 2, 3]);
    let h3 = hash_leaf_node(&vec![2, 3, 4]);
    let h4 = hash_leaf_node(&vec![3, 4, 5]);
    let h5 = hash_leaf_node(&vec![4, 5, 6]);
    let h12 = hash_branch_node(&[h1.as_ref(), h2.as_ref()].concat());
    let h34 = hash_branch_node(&[h3.as_ref(), h4.as_ref()].concat());
    let h1234 = hash_branch_node(&[h12.as_ref(), h34.as_ref()].concat());
    let h5up = hash_branch_node(h5.as_ref());
    let h5upup = hash_branch_node(h5up.as_ref());
    let h12345 = hash_branch_node(&[h1234.as_ref(), h5upup.as_ref()].concat());

    for i in 0_u8..5 {
        index.push(vec![i, i + 1, i + 2]);
    }

    let list_hash = HashTag::hash_list_node(index.len(), h12345);

    assert_eq!(index.list_hash(), list_hash);
    let range_proof = index.get_range_proof(4..5);

    assert_eq!(
        vec![4, 5, 6],
        *(range_proof.validate(list_hash, 5).unwrap()[0].1)
    );

    let serialized_proof = to_string(&range_proof).unwrap();
    let deserialized_proof: ListProof<Vec<u8>> = from_str(&serialized_proof).unwrap();
    assert_eq!(deserialized_proof, range_proof);

    if let ListProof::Right(left_hash1, right_proof1) = range_proof {
        assert_eq!(left_hash1, h1234);
        let unboxed_proof = *right_proof1;
        if let ListProof::Left(left_proof2, right_hash2) = unboxed_proof {
            assert!(right_hash2.is_none());
            let unboxed_proof = *left_proof2;
            if let ListProof::Left(_, right_hash3) = unboxed_proof {
                assert!(right_hash3.is_none());
            } else {
                assert!(false);
            }
        } else {
            assert!(false);
        }
    } else {
        assert!(false);
    }
}

fn simple_merkle_root(db: Box<dyn Database>) {
    let h1 = HashTag::hash_list_node(1, hash(&[0x0, 1]));
    let h2 = HashTag::hash_list_node(1, hash(&[0x0, 2]));

    let mut fork = db.fork();
    let mut index = ProofListIndex::new(IDX_NAME, &mut fork);
    assert_eq!(index.get(0), None);
    index.push(vec![1]);
    assert_eq!(index.list_hash(), h1);

    index.set(0, vec![2]);
    assert_eq!(index.list_hash(), h2);
}

fn same_merkle_root(db1: Box<dyn Database>, db2: Box<dyn Database>) {
    let mut fork1 = db1.fork();

    let mut i1 = ProofListIndex::new(IDX_NAME, &mut fork1);
    i1.push(vec![1]);
    i1.push(vec![2]);
    i1.push(vec![3]);
    i1.push(vec![4]);

    i1.set(0, vec![4]);
    i1.set(1, vec![7]);
    i1.set(2, vec![5]);
    i1.set(3, vec![1]);

    let mut fork2 = db2.fork();

    let mut i2 = ProofListIndex::new(IDX_NAME, &mut fork2);
    i2.push(vec![4]);
    i2.push(vec![7]);
    i2.push(vec![5]);
    i2.push(vec![1]);

    assert_eq!(i1.list_hash(), i2.list_hash());
}

#[derive(Serialize)]
struct ProofInfo<'a, V: Serialize + 'a> {
    merkle_root: Hash,
    list_length: u64,
    proof: &'a ListProof<V>,
    range_st: u64,
    range_end: u64,
}

mod memorydb_tests {
    use crate::{Database, MemoryDB};
    use std::path::Path;
    use tempfile::TempDir;

    fn create_database(_: &Path) -> Box<dyn Database> {
        Box::new(MemoryDB::new())
    }

    #[test]
    fn test_list_methods() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();
        let db = create_database(path);
        super::list_methods(db);
    }

    #[test]
    fn test_height() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();
        let db = create_database(path);
        super::height(db);
    }

    #[test]
    fn test_iter() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();
        let db = create_database(path);
        super::iter(db);
    }

    #[test]
    fn test_list_index_proof() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();
        let db = create_database(path);
        super::list_index_proof(db);
    }

    #[test]
    fn test_randomly_generate_proofs() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();
        let db = create_database(path);
        super::randomly_generate_proofs(db);
    }

    #[test]
    fn test_index_and_proof_roots() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();
        let db = create_database(path);
        super::index_and_proof_roots(db);
    }

    #[test]
    fn test_proof_illegal_lower_bound() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();
        let db = create_database(path);
        super::proof_illegal_lower_bound(db);
    }

    #[test]
    fn test_proof_illegal_bound_empty() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();
        let db = create_database(path);
        super::proof_illegal_bound_empty(db);
    }

    #[test]
    #[should_panic]
    fn test_proof_illegal_range() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();
        let db = create_database(path);
        super::proof_illegal_range(db);
    }

    #[test]
    fn test_proof_structure() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();
        let db = create_database(path);
        super::proof_structure(db);
    }

    #[test]
    fn test_simple_merkle_root() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();
        let db = create_database(path);
        super::simple_merkle_root(db);
    }

    #[test]
    fn test_same_merkle_root() {
        let dir1 = TempDir::new().unwrap();
        let path1 = dir1.path();
        let db1 = create_database(path1);
        let dir2 = TempDir::new().unwrap();
        let path2 = dir2.path();
        let db2 = create_database(path2);
        super::same_merkle_root(db1, db2);
    }
}

mod rocksdb_tests {
    use crate::{Database, DbOptions, RocksDB};
    use std::path::Path;
    use tempfile::TempDir;

    fn create_database(path: &Path) -> Box<dyn Database> {
        let opts = DbOptions::default();
        Box::new(RocksDB::open(path, &opts).unwrap())
    }

    #[test]
    fn test_list_methods() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();
        let db = create_database(path);
        super::list_methods(db);
    }

    #[test]
    fn test_height() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();
        let db = create_database(path);
        super::height(db);
    }

    #[test]
    fn test_iter() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();
        let db = create_database(path);
        super::iter(db);
    }

    #[test]
    fn test_list_index_proof() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();
        let db = create_database(path);
        super::list_index_proof(db);
    }

    #[test]
    fn test_randomly_generate_proofs() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();
        let db = create_database(path);
        super::randomly_generate_proofs(db);
    }

    #[test]
    fn test_index_and_proof_roots() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();
        let db = create_database(path);
        super::index_and_proof_roots(db);
    }

    #[test]
    fn test_proof_illegal_lower_bound() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();
        let db = create_database(path);
        super::proof_illegal_lower_bound(db);
    }

    #[test]
    fn test_proof_illegal_bound_empty() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();
        let db = create_database(path);
        super::proof_illegal_bound_empty(db);
    }

    #[test]
    #[should_panic]
    fn test_proof_illegal_range() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();
        let db = create_database(path);
        super::proof_illegal_range(db);
    }

    #[test]
    fn test_proof_structure() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();
        let db = create_database(path);
        super::proof_structure(db);
    }

    #[test]
    fn test_simple_merkle_root() {
        let dir = TempDir::new().unwrap();
        let path = dir.path();
        let db = create_database(path);
        super::simple_merkle_root(db);
    }

    #[test]
    fn test_same_merkle_root() {
        let dir1 = TempDir::new().unwrap();
        let path1 = dir1.path();
        let db1 = create_database(path1);
        let dir2 = TempDir::new().unwrap();
        let path2 = dir2.path();
        let db2 = create_database(path2);
        super::same_merkle_root(db1, db2);
    }
}

mod root_hash_tests {
    use crate::{hash::HashTag, Database, MemoryDB, ProofListIndex};
    use exonum_crypto::{self, Hash};

    /// Cross-verify `root_hash()` with `ProofListIndex` against expected root hash value.
    fn assert_root_hash_correct(hashes: &[Hash]) {
        let root_actual = HashTag::hash_list(hashes);
        let root_index = proof_list_index_root(hashes);
        assert_eq!(root_actual, root_index);
    }

    fn proof_list_index_root(hashes: &[Hash]) -> Hash {
        let db = MemoryDB::new();
        let mut fork = db.fork();
        let mut index = ProofListIndex::new("merkle_root", &mut fork);
        index.extend(hashes.iter().cloned());
        index.list_hash()
    }

    fn hash_list(bytes: &[&[u8]]) -> Vec<Hash> {
        bytes
            .iter()
            .map(|chunk| exonum_crypto::hash(chunk))
            .collect()
    }

    #[test]
    fn root_hash_single() {
        assert_root_hash_correct(&hash_list(&[b"1"]));
    }

    #[test]
    fn root_hash_even() {
        assert_root_hash_correct(&hash_list(&[b"1", b"2", b"3", b"4"]));
    }

    #[test]
    fn root_hash_odd() {
        assert_root_hash_correct(&hash_list(&[b"1", b"2", b"3", b"4", b"5"]));
    }

    #[test]
    fn root_hash_empty() {
        assert_root_hash_correct(&hash_list(&[]));
    }
}

#[test]
fn proof_of_absence_single() {
    let db = MemoryDB::new();
    let mut fork = db.fork();
    let mut list = ProofListIndex::new("absence", &mut fork);

    list.push(vec![1]);
    list.push(vec![2]);
    list.push(vec![3]);
    list.push(vec![4]);
    list.push(vec![5]);

    let root_hash =
        Hash::from_hex("5ba859b4d1799cb27ece9db8f7a76a50fc713a5d9d22f753eca42172996a88f9").unwrap();

    let non_existed_index = 6u64;
    let expected_hash = HashTag::hash_list_node(list.len(), root_hash);
    let proof = list.get_proof(non_existed_index);
    assert!(proof.validate(expected_hash, list.len()).is_ok());

    match proof {
        ListProof::Absent(proof) => {
            let actual_hash = HashTag::hash_list_node(proof.len(), proof.merkle_root());
            assert_eq!(expected_hash, actual_hash);
        }
        _ => {
            panic!("Unexpected proof {:?}", proof);
        }
    }
}
