// Copyright (c) 2022-2026 Alex Chi Z
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

#![allow(unused_variables)] // TODO(you): 实现该模块后删除此 lint
#![allow(dead_code)] // TODO(you): 实现该模块后删除此 lint

use std::ops::Bound;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

use anyhow::Result;
use bytes::Bytes;
use crossbeam_skiplist::SkipMap;
use ouroboros::self_referencing;

use crate::iterators::StorageIterator;
use crate::key::KeySlice;
use crate::table::SsTableBuilder;
use crate::wal::Wal;

/// 基于 crossbeam-skiplist 的基础内存表（memtable）。
///
/// 内存表的初始实现属于 week 1, day 1，并会在 week 1 和 week 2 的其他章节中
/// 逐步实现。
pub struct MemTable {
    map: Arc<SkipMap<Bytes, Bytes>>,
    wal: Option<Wal>,
    id: usize,
    approximate_size: Arc<AtomicUsize>,
}

/// 根据 `&[u8]` 的 bound 创建 `Bytes` 的 bound。
pub(crate) fn map_bound(bound: Bound<&[u8]>) -> Bound<Bytes> {
    match bound {
        Bound::Included(x) => Bound::Included(Bytes::copy_from_slice(x)),
        Bound::Excluded(x) => Bound::Excluded(Bytes::copy_from_slice(x)),
        Bound::Unbounded => Bound::Unbounded,
    }
}

impl MemTable {
    /// Week 1 Day 1：创建纯内存的内存表。
    pub fn create(_id: usize) -> Self {
        unimplemented!()
    }

    /// Week 2 Day 6：创建由预写日志（WAL）支撑的内存表。
    pub fn create_with_wal(_id: usize, _path: impl AsRef<Path>) -> Result<Self> {
        unimplemented!()
    }

    /// Week 2 Day 6：从预写日志中恢复内存表。
    pub fn recover_from_wal(_id: usize, _path: impl AsRef<Path>) -> Result<Self> {
        unimplemented!()
    }

    pub fn for_testing_put_slice(&self, key: &[u8], value: &[u8]) -> Result<()> {
        self.put(key, value)
    }

    pub fn for_testing_get_slice(&self, key: &[u8]) -> Option<Bytes> {
        self.get(key)
    }

    pub fn for_testing_scan_slice(
        &self,
        lower: Bound<&[u8]>,
        upper: Bound<&[u8]>,
    ) -> MemTableIterator {
        // 该函数只用于 week 1 的测试，所以在 week 3 的 key-ts 重构中，你不需要
        // 考虑 bound 的排除/包含逻辑。直接给 key-ts 对提供 `DEFAULT_TS` 作为
        // 时间戳即可。
        self.scan(lower, upper)
    }

    /// Week 1 Day 1：按键取值。
    pub fn get(&self, _key: &[u8]) -> Option<Bytes> {
        unimplemented!()
    }

    /// 把一个键值对写入内存表。
    ///
    /// Week 1 Day 1：把键值对写入内存中的 map。
    /// Week 2 Day 6：同时把数据追加到预写日志。
    /// Week 3 Day 5：让写操作走批量 WAL 实现。
    pub fn put(&self, _key: &[u8], _value: &[u8]) -> Result<()> {
        unimplemented!()
    }

    /// Week 3 Day 5：批量写入一批键值对。
    pub fn put_batch(&self, _data: &[(KeySlice, &[u8])]) -> Result<()> {
        unimplemented!()
    }

    pub fn sync_wal(&self) -> Result<()> {
        if let Some(ref wal) = self.wal {
            wal.sync()?;
        }
        Ok(())
    }

    /// 获取某个键区间上的迭代器。
    pub fn scan(&self, _lower: Bound<&[u8]>, _upper: Bound<&[u8]>) -> MemTableIterator {
        unimplemented!()
    }

    /// 把内存表落盘为 SSTable，在 week 1 day 6 实现。
    pub fn flush(&self, _builder: &mut SsTableBuilder) -> Result<()> {
        unimplemented!()
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn approximate_size(&self) -> usize {
        self.approximate_size
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// 只在关闭数据库时使用该函数
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

type SkipMapRangeIter<'a> =
    crossbeam_skiplist::map::Range<'a, Bytes, (Bound<Bytes>, Bound<Bytes>), Bytes, Bytes>;

/// `SkipMap` 某个区间上的迭代器。这是自引用结构，更多信息请参考 week 1, day 2
/// 章节。
///
/// 这是 week 1, day 2 的一部分。
#[self_referencing]
pub struct MemTableIterator {
    /// 保存对 skipmap 的引用。
    map: Arc<SkipMap<Bytes, Bytes>>,
    /// 保存一个 skipmap 迭代器，它引用 `MemTableIterator` 自身的生命周期。
    #[borrows(map)]
    #[not_covariant]
    iter: SkipMapRangeIter<'this>,
    /// 保存当前的键值对。
    item: (Bytes, Bytes),
}

impl StorageIterator for MemTableIterator {
    type KeyType<'a> = KeySlice<'a>;

    fn value(&self) -> &[u8] {
        unimplemented!()
    }

    fn key(&self) -> KeySlice<'_> {
        unimplemented!()
    }

    fn is_valid(&self) -> bool {
        unimplemented!()
    }

    fn next(&mut self) -> Result<()> {
        unimplemented!()
    }
}
