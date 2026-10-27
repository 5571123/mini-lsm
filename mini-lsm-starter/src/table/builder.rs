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

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;

use super::{BlockMeta, SsTable};
use crate::{block::BlockBuilder, key::KeySlice, lsm_storage::BlockCache};

/// 用键值对构建一个 SSTable。
pub struct SsTableBuilder {
    builder: BlockBuilder,
    first_key: Vec<u8>,
    last_key: Vec<u8>,
    data: Vec<u8>,
    pub(crate) meta: Vec<BlockMeta>,
    block_size: usize,
}

impl SsTableBuilder {
    /// 按目标块大小创建一个构建器。
    pub fn new(block_size: usize) -> Self {
        unimplemented!()
    }

    /// 向 SSTable 追加一个键值对。
    ///
    /// 注意：当前块写满时，你应切出一个新块。（`std::mem::replace` 在这里会很有用）
    pub fn add(&mut self, key: KeySlice, value: &[u8]) {
        unimplemented!()
    }

    /// 获取 SSTable 的预估大小。
    ///
    /// 由于数据块的数据量远多于元数据块，这里直接返回数据块的大小。
    pub fn estimated_size(&self) -> usize {
        unimplemented!()
    }

    /// 构建 SSTable 并写入指定路径。用 `FileObject` 结构操作磁盘对象。
    pub fn build(
        #[allow(unused_mut)] mut self,
        id: usize,
        block_cache: Option<Arc<BlockCache>>,
        path: impl AsRef<Path>,
    ) -> Result<SsTable> {
        unimplemented!()
    }

    #[cfg(test)]
    pub(crate) fn build_for_test(self, path: impl AsRef<Path>) -> Result<SsTable> {
        self.build(0, None, path)
    }
}
