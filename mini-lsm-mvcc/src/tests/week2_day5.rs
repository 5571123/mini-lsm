// Copyright (c) 2022-2025 Alex Chi Z
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::time::Duration;

use bytes::BufMut;
use tempfile::tempdir;

use crate::{
    compact::{
        CompactionOptions, LeveledCompactionOptions, SimpleLeveledCompactionOptions,
        TieredCompactionOptions,
    },
    lsm_storage::{LsmStorageOptions, MiniLsm},
    tests::harness::dump_files_in_dir,
};

#[test]
fn test_integration_leveled() {
    test_integration(CompactionOptions::Leveled(LeveledCompactionOptions {
        level_size_multiplier: 2,
        level0_file_num_compaction_trigger: 2,
        max_levels: 3,
        base_level_size_mb: 1,
    }))
}

#[test]
fn test_integration_tiered() {
    test_integration(CompactionOptions::Tiered(TieredCompactionOptions {
        num_tiers: 3,
        max_size_amplification_percent: 200,
        size_ratio: 1,
        min_merge_width: 3,
        max_merge_width: None,
    }))
}

#[test]
fn test_integration_simple() {
    test_integration(CompactionOptions::Simple(SimpleLeveledCompactionOptions {
        size_ratio_percent: 200,
        level0_file_num_compaction_trigger: 2,
        max_levels: 3,
    }));
}

/// 构造这样的存储状态：base_level 含 2 个 SSTable（目标大小 2MB，每个 SSTable 为 1MB）。
/// 该配置的效果是：压缩会生成一个含多个 SSTable 的新下层，
/// 而 leveled 压缩应正确处理这种情况：这些文件可能未按首键排序，
/// 也不应在 `apply_compaction_result` 函数内部排序，因为在清单恢复期间调用该函数时，
/// 我们还没有加载任何实际的 SSTable。
#[test]
fn test_multiple_compacted_ssts_leveled() {
    let compaction_options = CompactionOptions::Leveled(LeveledCompactionOptions {
        level_size_multiplier: 4,
        level0_file_num_compaction_trigger: 2,
        max_levels: 2,
        base_level_size_mb: 2,
    });

    let lsm_storage_options = LsmStorageOptions::default_for_week2_test(compaction_options.clone());

    let dir = tempdir().unwrap();
    let storage = MiniLsm::open(&dir, lsm_storage_options).unwrap();

    // 写入约 10MB 数据，确保至少触发一次按优先级调度的压缩。
    // 插入 500 个键值对，每对 2KB
    for i in 0..500 {
        let (key, val) = key_value_pair_with_target_size(i, 20 * 1024);
        storage.put(&key, &val).unwrap();
    }

    let mut prev_snapshot = storage.inner.state.read().clone();
    while {
        std::thread::sleep(Duration::from_secs(1));
        let snapshot = storage.inner.state.read().clone();
        let to_cont = prev_snapshot.levels != snapshot.levels
            || prev_snapshot.l0_sstables != snapshot.l0_sstables;
        prev_snapshot = snapshot;
        to_cont
    } {
        println!("waiting for compaction to converge");
    }

    storage.close().unwrap();
    assert!(storage.inner.state.read().memtable.is_empty());
    assert!(storage.inner.state.read().imm_memtables.is_empty());

    storage.dump_structure();
    drop(storage);
    dump_files_in_dir(&dir);

    let storage = MiniLsm::open(
        &dir,
        LsmStorageOptions::default_for_week2_test(compaction_options.clone()),
    )
    .unwrap();

    for i in 0..500 {
        let (key, val) = key_value_pair_with_target_size(i, 20 * 1024);
        assert_eq!(&storage.get(&key).unwrap().unwrap()[..], &val);
    }
}

fn test_integration(compaction_options: CompactionOptions) {
    let dir = tempdir().unwrap();
    let storage = MiniLsm::open(
        &dir,
        LsmStorageOptions::default_for_week2_test(compaction_options.clone()),
    )
    .unwrap();
    for i in 0..=20 {
        storage.put(b"0", format!("v{}", i).as_bytes()).unwrap();
        if i % 2 == 0 {
            storage.put(b"1", format!("v{}", i).as_bytes()).unwrap();
        } else {
            storage.delete(b"1").unwrap();
        }
        if i % 2 == 1 {
            storage.put(b"2", format!("v{}", i).as_bytes()).unwrap();
        } else {
            storage.delete(b"2").unwrap();
        }
        storage
            .inner
            .force_freeze_memtable(&storage.inner.state_lock.lock())
            .unwrap();
    }
    storage.close().unwrap();
    // 确保所有 SSTable 都已落盘
    assert!(storage.inner.state.read().memtable.is_empty());
    assert!(storage.inner.state.read().imm_memtables.is_empty());
    storage.dump_structure();
    drop(storage);
    dump_files_in_dir(&dir);

    let storage = MiniLsm::open(
        &dir,
        LsmStorageOptions::default_for_week2_test(compaction_options.clone()),
    )
    .unwrap();
    assert_eq!(&storage.get(b"0").unwrap().unwrap()[..], b"v20".as_slice());
    assert_eq!(&storage.get(b"1").unwrap().unwrap()[..], b"v20".as_slice());
    assert_eq!(storage.get(b"2").unwrap(), None);
}

/// 创建键和值均为目标字节大小的键值对
fn key_value_pair_with_target_size(seed: i32, target_size_byte: usize) -> (Vec<u8>, Vec<u8>) {
    let mut key = vec![0; target_size_byte - 4];
    key.put_i32(seed);

    let mut val = vec![0; target_size_byte - 4];
    val.put_i32(seed);

    (key, val)
}
