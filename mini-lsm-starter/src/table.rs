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

pub(crate) mod bloom;
mod builder;
mod iterator;

use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
pub use builder::SsTableBuilder;
use bytes::Buf;
pub use iterator::SsTableIterator;

use crate::block::Block;
use crate::key::{KeyBytes, KeySlice};
use crate::lsm_storage::BlockCache;

use self::bloom::Bloom;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockMeta {
    /// 该数据块的偏移。
    pub offset: usize,
    /// 该数据块的第一个键。
    pub first_key: KeyBytes,
    /// 该数据块的最后一个键。
    pub last_key: KeyBytes,
}

impl BlockMeta {
    /// 把块元数据编码到缓冲区中。
    /// 你可以往缓冲区里添加额外字段，
    /// 以便将来从同一缓冲区解码时能追踪 `first_key`。
    pub fn encode_block_meta(
        block_meta: &[BlockMeta],
        #[allow(clippy::ptr_arg)] // 完成后删除这个 allow
        buf: &mut Vec<u8>,
    ) {
        unimplemented!()
    }

    /// 从缓冲区中解码块元数据。
    pub fn decode_block_meta(buf: impl Buf) -> Vec<BlockMeta> {
        unimplemented!()
    }
}

/// 一个文件对象。
pub struct FileObject(Option<File>, u64);

impl FileObject {
    pub fn read(&self, offset: u64, len: u64) -> Result<Vec<u8>> {
        let mut data = vec![0; len as usize];
        read_exact_at(self.0.as_ref().unwrap(), &mut data, offset)?;
        Ok(data)
    }

    pub fn size(&self) -> u64 {
        self.1
    }

    /// 创建一个新的文件对象（day 2），并把文件写入磁盘（day 4）。
    pub fn create(path: &Path, data: Vec<u8>) -> Result<Self> {
        std::fs::write(path, &data)?;
        // Windows 下 `FlushFileBuffers` 需要写权限，只读句柄会返回「拒绝访问」，故以读写方式打开。
        File::options()
            .read(true)
            .write(true)
            .open(path)?
            .sync_all()?;
        Ok(FileObject(
            Some(File::options().read(true).write(false).open(path)?),
            data.len() as u64,
        ))
    }

    pub fn open(path: &Path) -> Result<Self> {
        let file = File::options().read(true).write(false).open(path)?;
        let size = file.metadata()?.len();
        Ok(FileObject(Some(file), size))
    }
}

/// 按偏移读取数据：Unix 使用 `pread`，Windows 使用 `seek_read`。
///
/// 注意不要用「先 seek 再 read」实现：`SsTable` 会被多个线程共享读取，共享的文件游标会被并发修改。
#[cfg(unix)]
fn read_exact_at(file: &File, buf: &mut [u8], offset: u64) -> std::io::Result<()> {
    use std::os::unix::fs::FileExt;
    file.read_exact_at(buf, offset)
}

#[cfg(windows)]
fn read_exact_at(file: &File, mut buf: &mut [u8], mut offset: u64) -> std::io::Result<()> {
    use std::os::windows::fs::FileExt;
    while !buf.is_empty() {
        match file.seek_read(buf, offset) {
            Ok(0) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "failed to fill whole buffer",
                ));
            }
            Ok(n) => {
                offset += n as u64;
                let rest = buf;
                buf = &mut rest[n..];
            }
            Err(err) if err.kind() == std::io::ErrorKind::Interrupted => {}
            Err(err) => return Err(err),
        }
    }
    Ok(())
}

/// 一个 SSTable。
pub struct SsTable {
    /// SsTable 实际的存储单元，格式如上所述。
    pub(crate) file: FileObject,
    /// 保存数据块信息的元数据块。
    pub(crate) block_meta: Vec<BlockMeta>,
    /// 元数据块在 `file` 中的起始偏移。
    pub(crate) block_meta_offset: usize,
    id: usize,
    block_cache: Option<Arc<BlockCache>>,
    first_key: KeyBytes,
    last_key: KeyBytes,
    pub(crate) bloom: Option<Bloom>,
    /// 该 SST 中存储的最大时间戳，在 week 3 实现。
    max_ts: u64,
}

impl SsTable {
    #[cfg(test)]
    pub(crate) fn open_for_test(file: FileObject) -> Result<Self> {
        Self::open(0, None, file)
    }

    /// 从文件打开 SSTable。
    pub fn open(id: usize, block_cache: Option<Arc<BlockCache>>, file: FileObject) -> Result<Self> {
        unimplemented!()
    }

    /// 创建一个只含 `first_key` 与 `last_key` 元数据的模拟 SST
    pub fn create_meta_only(
        id: usize,
        file_size: u64,
        first_key: KeyBytes,
        last_key: KeyBytes,
    ) -> Self {
        Self {
            file: FileObject(None, file_size),
            block_meta: vec![],
            block_meta_offset: 0,
            id,
            block_cache: None,
            first_key,
            last_key,
            bloom: None,
            max_ts: 0,
        }
    }

    /// 从磁盘读取一个块。
    pub fn read_block(&self, block_idx: usize) -> Result<Arc<Block>> {
        unimplemented!()
    }

    /// 从磁盘读取一个块，并经过块缓存。(Day 4)
    pub fn read_block_cached(&self, block_idx: usize) -> Result<Arc<Block>> {
        unimplemented!()
    }

    /// 找到可能包含 `key` 的块。
    /// 注意：你可以利用 `BlockMeta` 中保存的 `first_key`。
    /// 你也可以假设相邻块中存放的键值对是有序的。
    pub fn find_block_idx(&self, key: KeySlice) -> usize {
        unimplemented!()
    }

    /// 获取数据块的数量。
    pub fn num_of_blocks(&self) -> usize {
        self.block_meta.len()
    }

    pub fn first_key(&self) -> &KeyBytes {
        &self.first_key
    }

    pub fn last_key(&self) -> &KeyBytes {
        &self.last_key
    }

    pub fn table_size(&self) -> u64 {
        self.file.1
    }

    pub fn sst_id(&self) -> usize {
        self.id
    }

    pub fn max_ts(&self) -> u64 {
        self.max_ts
    }
}
