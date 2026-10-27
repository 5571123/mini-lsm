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

use std::sync::Arc;

use anyhow::Result;

use super::SsTable;
use crate::{block::BlockIterator, iterators::StorageIterator, key::KeySlice};

/// SSTable 内容上的迭代器。
pub struct SsTableIterator {
    table: Arc<SsTable>,
    blk_iter: BlockIterator,
    blk_idx: usize,
}

impl SsTableIterator {
    /// 创建新迭代器，并定位到第一个数据块中的第一个键值对。
    pub fn create_and_seek_to_first(table: Arc<SsTable>) -> Result<Self> {
        unimplemented!()
    }

    /// 定位到第一个数据块中的第一个键值对。
    pub fn seek_to_first(&mut self) -> Result<()> {
        unimplemented!()
    }

    /// 创建新迭代器，并定位到第一个键 >= `key` 的键值对。
    pub fn create_and_seek_to_key(table: Arc<SsTable>, key: KeySlice) -> Result<Self> {
        unimplemented!()
    }

    /// 定位到第一个键 >= `key` 的键值对。
    /// 注意：实现该函数时，你大概需要回顾讲义中的详细说明。
    pub fn seek_to_key(&mut self, key: KeySlice) -> Result<()> {
        unimplemented!()
    }
}

impl StorageIterator for SsTableIterator {
    type KeyType<'a> = KeySlice<'a>;

    /// 返回底层块迭代器当前持有的 `key`。
    fn key(&self) -> KeySlice<'_> {
        unimplemented!()
    }

    /// 返回底层块迭代器当前持有的 `value`。
    fn value(&self) -> &[u8] {
        unimplemented!()
    }

    /// 返回当前块迭代器是否有效。
    fn is_valid(&self) -> bool {
        unimplemented!()
    }

    /// 移动到块中的下一个 `key`。
    /// 注意：移动之后你可能需要检查当前块迭代器是否有效。
    fn next(&mut self) -> Result<()> {
        unimplemented!()
    }
}
