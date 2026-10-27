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

use std::ops::Bound;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

use anyhow::Result;
use bytes::Bytes;
use crossbeam_skiplist::SkipMap;
use crossbeam_skiplist::map::Entry;
use ouroboros::self_referencing;

use crate::iterators::StorageIterator;
use crate::key::{KeyBytes, KeySlice, TS_DEFAULT, TS_RANGE_BEGIN, TS_RANGE_END};
use crate::table::SsTableBuilder;
use crate::wal::Wal;

/// 基于 crossbeam-skiplist 的基本内存表（memtable）。
///
/// 内存表的初始实现在 Week 1 Day 1 中完成。它会在 Week 1 和 Week 2 的其他
/// 章节中逐步实现完整。
pub struct MemTable {
    pub(crate) map: Arc<SkipMap<KeyBytes, Bytes>>,
    wal: Option<Wal>,
    id: usize,
    approximate_size: Arc<AtomicUsize>,
}

/// 由 `&[u8]` 的边界（bound）创建一个 `Bytes` 的边界。
pub(crate) fn map_bound(bound: Bound<&[u8]>) -> Bound<Bytes> {
    match bound {
        Bound::Included(x) => Bound::Included(Bytes::copy_from_slice(x)),
        Bound::Excluded(x) => Bound::Excluded(Bytes::copy_from_slice(x)),
        Bound::Unbounded => Bound::Unbounded,
    }
}

/// 由 `KeySlice` 的边界创建一个 `Bytes` 的边界。
pub(crate) fn map_key_bound(bound: Bound<KeySlice>) -> Bound<KeyBytes> {
    match bound {
        Bound::Included(x) => Bound::Included(KeyBytes::from_bytes_with_ts(
            Bytes::copy_from_slice(x.key_ref()),
            x.ts(),
        )),
        Bound::Excluded(x) => Bound::Excluded(KeyBytes::from_bytes_with_ts(
            Bytes::copy_from_slice(x.key_ref()),
            x.ts(),
        )),
        Bound::Unbounded => Bound::Unbounded,
    }
}

/// 由 `&[u8]` 的边界创建一个 `KeySlice` 的边界。
pub(crate) fn map_key_bound_plus_ts<'a>(
    lower: Bound<&'a [u8]>,
    upper: Bound<&'a [u8]>,
    ts: u64,
) -> (Bound<KeySlice<'a>>, Bound<KeySlice<'a>>) {
    (
        match lower {
            Bound::Included(x) => Bound::Included(KeySlice::from_slice_with_ts(x, ts)),
            Bound::Excluded(x) => Bound::Excluded(KeySlice::from_slice_with_ts(x, TS_RANGE_END)),
            Bound::Unbounded => Bound::Unbounded,
        },
        match upper {
            Bound::Included(x) => {
                // 注意 ts 是按降序排列的，但 MVCC 扫描需要全部历史版本，
                // 这样当某个键在当前 ts 下没有被更新时，才能访问到最新的那个键。
                // 因此需要一直扫描到 ts 0。
                Bound::Included(KeySlice::from_slice_with_ts(x, TS_RANGE_END))
            }
            Bound::Excluded(x) => Bound::Excluded(KeySlice::from_slice_with_ts(x, TS_RANGE_BEGIN)),
            Bound::Unbounded => Bound::Unbounded,
        },
    )
}

impl MemTable {
    /// 创建一个新的内存表。
    pub fn create(id: usize) -> Self {
        Self {
            id,
            map: Arc::new(SkipMap::new()),
            wal: None,
            approximate_size: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// 创建一个带 WAL 的新内存表
    pub fn create_with_wal(id: usize, path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            id,
            map: Arc::new(SkipMap::new()),
            wal: Some(Wal::create(path.as_ref())?),
            approximate_size: Arc::new(AtomicUsize::new(0)),
        })
    }

    /// 由 WAL 恢复出一个内存表
    pub fn recover_from_wal(id: usize, path: impl AsRef<Path>) -> Result<Self> {
        let map = Arc::new(SkipMap::new());
        Ok(Self {
            id,
            wal: Some(Wal::recover(path.as_ref(), &map)?),
            map,
            approximate_size: Arc::new(AtomicUsize::new(0)),
        })
    }

    /// 按键取值。在 Week 3 中不应使用。
    pub fn get(&self, key: KeySlice) -> Option<Bytes> {
        let key_bytes = KeyBytes::from_bytes_with_ts(
            Bytes::from_static(unsafe { std::mem::transmute::<&[u8], &[u8]>(key.key_ref()) }),
            key.ts(),
        );
        self.map.get(&key_bytes).map(|e| e.value().clone())
    }

    pub fn for_testing_put_slice(&self, key: &[u8], value: &[u8]) -> Result<()> {
        self.put(KeySlice::from_slice_with_ts(key, TS_DEFAULT), value)
    }

    pub fn for_testing_get_slice(&self, key: &[u8]) -> Option<Bytes> {
        self.get(KeySlice::from_slice_with_ts(key, TS_DEFAULT))
    }

    pub fn for_testing_scan_slice(
        &self,
        lower: Bound<&[u8]>,
        upper: Bound<&[u8]>,
    ) -> MemTableIterator {
        self.scan(
            lower.map(|x| KeySlice::from_slice_with_ts(x, TS_DEFAULT)),
            upper.map(|x| KeySlice::from_slice_with_ts(x, TS_DEFAULT)),
        )
    }

    /// 把一个键值对写入内存表。
    ///
    /// 在 Week 1 Day 1 中，只需把键值对放进 skipmap。
    /// 在 Week 2 Day 6 中，还要把数据写入 WAL 并落盘。
    /// 在 Week 3 Day 5 中，把单条写入改为走批量 WAL 实现。
    pub fn put(&self, key: KeySlice, value: &[u8]) -> Result<()> {
        self.put_batch(&[(key, value)])
    }

    /// 在 Week 3 Day 5 中实现。
    pub fn put_batch(&self, data: &[(KeySlice, &[u8])]) -> Result<()> {
        if let Some(ref wal) = self.wal {
            wal.put_batch(data)?;
        }

        let mut estimated_size = 0;
        for (key, value) in data {
            estimated_size += key.raw_len() + value.len();
            self.map.insert(
                key.to_key_vec().into_key_bytes(),
                Bytes::copy_from_slice(value),
            );
        }
        self.approximate_size
            .fetch_add(estimated_size, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    pub fn sync_wal(&self) -> Result<()> {
        if let Some(ref wal) = self.wal {
            wal.sync()?;
        }
        Ok(())
    }

    /// 获取一个键范围内的迭代器。
    pub fn scan(&self, lower: Bound<KeySlice>, upper: Bound<KeySlice>) -> MemTableIterator {
        let (lower, upper) = (map_key_bound(lower), map_key_bound(upper));
        let mut iter = MemTableIteratorBuilder {
            map: self.map.clone(),
            iter_builder: |map| map.range((lower, upper)),
            item: (KeyBytes::new(), Bytes::new()),
        }
        .build();
        iter.next().unwrap();
        iter
    }

    /// 把内存表落盘为 SSTable。在 Week 1 Day 6 中实现。
    pub fn flush(&self, builder: &mut SsTableBuilder) -> Result<()> {
        for entry in self.map.iter() {
            builder.add(entry.key().as_key_slice(), &entry.value()[..]);
        }
        Ok(())
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

type SkipMapRangeIter<'a> = crossbeam_skiplist::map::Range<
    'a,
    KeyBytes,
    (Bound<KeyBytes>, Bound<KeyBytes>),
    KeyBytes,
    Bytes,
>;

/// 在 `SkipMap` 的一段范围上迭代。这是一个自引用结构，更多信息请参阅
/// Week 1 Day 2 章节。
///
/// 这是 Week 1 Day 2 的一部分。
#[self_referencing]
pub struct MemTableIterator {
    /// 保存 skipmap 的引用。
    map: Arc<SkipMap<KeyBytes, Bytes>>,
    /// 保存一个指向 `MemTableIterator` 自身生命周期的 skipmap 迭代器。
    #[borrows(map)]
    #[not_covariant]
    iter: SkipMapRangeIter<'this>,
    /// 保存当前的键值对。
    item: (KeyBytes, Bytes),
}

impl MemTableIterator {
    fn entry_to_item(entry: Option<Entry<'_, KeyBytes, Bytes>>) -> (KeyBytes, Bytes) {
        entry
            .map(|x| (x.key().clone(), x.value().clone()))
            .unwrap_or_else(|| (KeyBytes::new(), Bytes::new()))
    }
}

impl StorageIterator for MemTableIterator {
    type KeyType<'a> = KeySlice<'a>;

    fn value(&self) -> &[u8] {
        &self.borrow_item().1[..]
    }

    fn key(&self) -> KeySlice<'_> {
        self.borrow_item().0.as_key_slice()
    }

    fn is_valid(&self) -> bool {
        !self.borrow_item().0.is_empty()
    }

    fn next(&mut self) -> Result<()> {
        let entry = self.with_iter_mut(|iter| MemTableIterator::entry_to_item(iter.next()));
        self.with_mut(|x| *x.item = entry);
        Ok(())
    }
}
