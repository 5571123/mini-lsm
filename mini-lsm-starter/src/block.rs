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

mod builder;
mod iterator;

pub use builder::BlockBuilder;
use bytes::Bytes;
pub use iterator::BlockIterator;

/// 块（block）是 LSM tree 中读取和缓存的最小单位，是一批有序的键值对。
pub struct Block {
    pub(crate) data: Vec<u8>,
    pub(crate) offsets: Vec<u16>,
}

impl Block {
    /// 把内部数据编码成课程中给出的数据布局
    /// 注意：你需要复查输出中是否漏掉了某个应有的字段
    pub fn encode(&self) -> Bytes {
        unimplemented!()
    }

    /// 按数据布局解码，把输入的 `data` 转换成单个 `Block`
    pub fn decode(data: &[u8]) -> Self {
        unimplemented!()
    }
}
