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

use bytes::BufMut;

use crate::key::{KeySlice, KeyVec};

use super::{Block, SIZEOF_U16};

/// 构建一个块（block）。
pub struct BlockBuilder {
    /// 每个键值对条目的偏移量。
    offsets: Vec<u16>,
    /// 块中所有序列化后的键值对。
    data: Vec<u8>,
    /// 期望的块大小。
    block_size: usize,
    /// 块中的第一个键
    first_key: KeyVec,
}

fn compute_overlap(first_key: KeySlice, key: KeySlice) -> usize {
    let mut i = 0;
    loop {
        if i >= first_key.key_len() || i >= key.key_len() {
            break;
        }
        if first_key.key_ref()[i] != key.key_ref()[i] {
            break;
        }
        i += 1;
    }
    i
}

impl BlockBuilder {
    /// 创建一个新的块构建器。
    pub fn new(block_size: usize) -> Self {
        Self {
            offsets: Vec::new(),
            data: Vec::new(),
            block_size,
            first_key: KeyVec::new(),
        }
    }

    fn estimated_size(&self) -> usize {
        SIZEOF_U16 /* 块中键值对的个数 */ +  self.offsets.len() * SIZEOF_U16 /* 偏移量 */ + self.data.len()
        // 键值对
    }

    /// 向块中添加一个键值对。当块已满时返回 false。
    #[must_use]
    pub fn add(&mut self, key: KeySlice, value: &[u8]) -> bool {
        assert!(!key.is_empty(), "key must not be empty");
        let Ok(key_len) = u16::try_from(key.key_len()) else {
            return false;
        };
        let Ok(value_len) = u16::try_from(value.len()) else {
            return false;
        };
        let entry_size = key
            .raw_len()
            .saturating_add(value.len())
            .saturating_add(SIZEOF_U16 * 3);
        let block_is_full = self.estimated_size().saturating_add(entry_size) > self.block_size;
        let offset_is_full = self.data.len() > usize::from(u16::MAX);
        let count_is_full = self.offsets.len() >= usize::from(u16::MAX);
        if !self.is_empty() && (block_is_full || offset_is_full || count_is_full) {
            return false;
        }
        // 把数据的偏移量加入偏移量数组。
        let Ok(offset) = u16::try_from(self.data.len()) else {
            return false;
        };
        self.offsets.push(offset);
        let overlap = compute_overlap(self.first_key.as_key_slice(), key);
        let Ok(overlap) = u16::try_from(overlap) else {
            return false;
        };
        // 编码键的公共前缀长度。
        self.data.put_u16(overlap);
        // 编码键长度。
        self.data.put_u16(key_len - overlap);
        // 编码键内容。
        self.data.put(&key.key_ref()[usize::from(overlap)..]);
        // 编码键的时间戳（ts）
        self.data.put_u64(key.ts());
        // 编码值长度。
        self.data.put_u16(value_len);
        // 编码值内容。
        self.data.put(value);

        if self.first_key.is_empty() {
            self.first_key = key.to_key_vec();
        }

        true
    }

    /// 检查块中是否没有任何键值对。
    pub fn is_empty(&self) -> bool {
        self.offsets.is_empty()
    }

    /// 完成块的构建。
    pub fn build(self) -> Block {
        if self.is_empty() {
            panic!("block should not be empty");
        }
        Block {
            data: self.data,
            offsets: self.offsets,
        }
    }
}
