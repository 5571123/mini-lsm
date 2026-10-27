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

use crate::key::{KeySlice, KeyVec};

use super::Block;

/// 构建一个块。
pub struct BlockBuilder {
    /// 每个键值对条目的偏移。
    offsets: Vec<u16>,
    /// 块内所有已序列化的键值对。
    data: Vec<u8>,
    /// 期望的块大小。
    block_size: usize,
    /// 块中的第一个键
    first_key: KeyVec,
}

impl BlockBuilder {
    /// 创建一个新的块构建器。
    pub fn new(block_size: usize) -> Self {
        unimplemented!()
    }

    /// 向块中追加一个键值对。块已满时返回 false。
    /// 操作二进制数据时，`bytes::BufMut` trait 会很有用。
    #[must_use]
    pub fn add(&mut self, key: KeySlice, value: &[u8]) -> bool {
        unimplemented!()
    }

    /// 检查块中是否没有任何键值对。
    pub fn is_empty(&self) -> bool {
        unimplemented!()
    }

    /// 完成块的构建。
    pub fn build(self) -> Block {
        unimplemented!()
    }
}
