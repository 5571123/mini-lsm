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

use crate::key::{KeySlice, KeyVec};

use super::Block;

/// 在块上迭代。
pub struct BlockIterator {
    /// 内部的 `Block`，用 `Arc` 包裹
    block: Arc<Block>,
    /// 当前键，为空表示迭代器无效
    key: KeyVec,
    /// 当前值在 block.data 中的范围，与当前键对应
    value_range: (usize, usize),
    /// 当前键值对的下标，应处于 [0, num_of_elements) 范围内
    idx: usize,
    /// 块中的第一个键
    first_key: KeyVec,
}

impl BlockIterator {
    fn new(block: Arc<Block>) -> Self {
        Self {
            block,
            key: KeyVec::new(),
            value_range: (0, 0),
            idx: 0,
            first_key: KeyVec::new(),
        }
    }

    /// 创建块迭代器，并定位到第一个条目。
    pub fn create_and_seek_to_first(block: Arc<Block>) -> Self {
        unimplemented!()
    }

    /// 创建块迭代器，并定位到第一个 >= `key` 的键。
    pub fn create_and_seek_to_key(block: Arc<Block>, key: KeySlice) -> Self {
        unimplemented!()
    }

    /// 返回当前条目的键。
    pub fn key(&self) -> KeySlice<'_> {
        unimplemented!()
    }

    /// 返回当前条目的值。
    pub fn value(&self) -> &[u8] {
        unimplemented!()
    }

    /// 迭代器有效时返回 true。
    /// 注意：你可以利用 `key` 来判断
    pub fn is_valid(&self) -> bool {
        unimplemented!()
    }

    /// 定位到块中的第一个键。
    pub fn seek_to_first(&mut self) {
        unimplemented!()
    }

    /// 移动到块中的下一个键。
    pub fn next(&mut self) {
        unimplemented!()
    }

    /// 定位到第一个 >= `key` 的键。
    /// 注意：你应假设调用方追加键值对时已按有序顺序写入。
    pub fn seek_to_key(&mut self, key: KeySlice) {
        unimplemented!()
    }
}
