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

pub(crate) mod bloom;
mod builder;
mod iterator;

use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail, ensure};
pub use builder::SsTableBuilder;
use bytes::BufMut;
pub use iterator::SsTableIterator;

use crate::block::Block;
use crate::key::{KeyBytes, KeySlice};
use crate::lsm_storage::BlockCache;

use self::bloom::Bloom;

fn take_bytes<'a>(buf: &mut &'a [u8], len: usize, what: &str) -> Result<&'a [u8]> {
    ensure!(buf.len() >= len, "{what} is truncated");
    let (value, rest) = buf.split_at(len);
    *buf = rest;
    Ok(value)
}

fn take_u16(buf: &mut &[u8], what: &str) -> Result<u16> {
    let value = take_bytes(buf, std::mem::size_of::<u16>(), what)?;
    Ok(u16::from_be_bytes([value[0], value[1]]))
}

fn take_u32(buf: &mut &[u8], what: &str) -> Result<u32> {
    let value = take_bytes(buf, std::mem::size_of::<u32>(), what)?;
    Ok(u32::from_be_bytes([value[0], value[1], value[2], value[3]]))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockMeta {
    /// 该数据块的偏移量。
    pub offset: usize,
    /// 数据块的第一个键。
    pub first_key: KeyBytes,
    /// 数据块的最后一个键。
    pub last_key: KeyBytes,
}

impl BlockMeta {
    /// 把块元数据编码到缓冲区。
    pub fn encode_block_meta(block_meta: &[BlockMeta], buf: &mut Vec<u8>) -> Result<()> {
        let mut estimated_size = std::mem::size_of::<u32>();
        for meta in block_meta {
            // 偏移量的大小
            estimated_size += std::mem::size_of::<u32>();
            // 键长度的大小
            estimated_size += std::mem::size_of::<u16>();
            // 实际键的大小
            estimated_size += meta.first_key.len();
            // 键长度的大小
            estimated_size += std::mem::size_of::<u16>();
            // 实际键的大小
            estimated_size += meta.last_key.len();
        }
        estimated_size += std::mem::size_of::<u32>();
        // 预留空间以提升性能，在写入数据量较大时尤其明显
        buf.reserve(estimated_size);
        let original_len = buf.len();
        buf.put_u32(u32::try_from(block_meta.len()).context("too many SST blocks")?);
        for meta in block_meta {
            buf.put_u32(u32::try_from(meta.offset).context("SST block offset is too large")?);
            buf.put_u16(u16::try_from(meta.first_key.len()).context("SST first key is too large")?);
            buf.put_slice(meta.first_key.raw_ref());
            buf.put_u16(u16::try_from(meta.last_key.len()).context("SST last key is too large")?);
            buf.put_slice(meta.last_key.raw_ref());
        }
        buf.put_u32(crc32fast::hash(&buf[original_len + 4..]));
        assert_eq!(estimated_size, buf.len() - original_len);
        Ok(())
    }

    /// 从缓冲区解码块元数据。
    pub fn decode_block_meta(buf: &[u8]) -> Result<Vec<BlockMeta>> {
        ensure!(
            buf.len() >= std::mem::size_of::<u32>() * 2,
            "SST block metadata is truncated"
        );
        let checksum_offset = buf.len() - std::mem::size_of::<u32>();
        let expected_checksum = u32::from_be_bytes([
            buf[checksum_offset],
            buf[checksum_offset + 1],
            buf[checksum_offset + 2],
            buf[checksum_offset + 3],
        ]);
        let checksum = crc32fast::hash(&buf[std::mem::size_of::<u32>()..checksum_offset]);
        ensure!(expected_checksum == checksum, "meta checksum mismatched");

        let mut cursor = buf;
        let mut block_meta = Vec::new();
        let num = take_u32(&mut cursor, "SST block count")? as usize;
        let minimum_entry_size = std::mem::size_of::<u32>() + std::mem::size_of::<u16>() * 2;
        ensure!(
            num <= checksum_offset.saturating_sub(std::mem::size_of::<u32>()) / minimum_entry_size,
            "SST block count exceeds the metadata length"
        );
        for _ in 0..num {
            let offset = take_u32(&mut cursor, "SST block offset")? as usize;
            let first_key_len = take_u16(&mut cursor, "SST first-key length")? as usize;
            ensure!(first_key_len > 0, "SST first key is empty");
            let first_key = KeyBytes::from_bytes(
                take_bytes(&mut cursor, first_key_len, "SST first key")?
                    .to_vec()
                    .into(),
            );
            let last_key_len = take_u16(&mut cursor, "SST last-key length")? as usize;
            ensure!(last_key_len > 0, "SST last key is empty");
            let last_key = KeyBytes::from_bytes(
                take_bytes(&mut cursor, last_key_len, "SST last key")?
                    .to_vec()
                    .into(),
            );
            block_meta.push(BlockMeta {
                offset,
                first_key,
                last_key,
            });
        }
        ensure!(
            cursor.len() == std::mem::size_of::<u32>(),
            "SST block metadata has trailing or missing bytes"
        );
        let stored_checksum = take_u32(&mut cursor, "SST metadata checksum")?;
        ensure!(stored_checksum == checksum, "meta checksum mismatched");

        Ok(block_meta)
    }
}

/// 一个文件对象。
pub struct FileObject(Option<File>, u64);

impl FileObject {
    pub fn read(&self, offset: u64, len: u64) -> Result<Vec<u8>> {
        let end = offset
            .checked_add(len)
            .context("file read range overflow")?;
        ensure!(end <= self.1, "file read range is out of bounds");
        let len = usize::try_from(len).context("file read is too large")?;
        let mut data = vec![0; len];
        let file = self
            .0
            .as_ref()
            .context("cannot read a metadata-only file object")?;
        read_exact_at(file, &mut data, offset)?;
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
            u64::try_from(data.len()).context("SST file is too large")?,
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
    /// 元数据块在 `file` 中的起始偏移量。
    pub(crate) block_meta_offset: usize,
    id: usize,
    block_cache: Option<Arc<BlockCache>>,
    first_key: KeyBytes,
    last_key: KeyBytes,
    pub(crate) bloom: Option<Bloom>,
    max_ts: u64,
}
impl SsTable {
    #[cfg(test)]
    pub(crate) fn open_for_test(file: FileObject) -> Result<Self> {
        Self::open(0, None, file)
    }

    /// 从文件打开 SSTable。
    pub fn open(id: usize, block_cache: Option<Arc<BlockCache>>, file: FileObject) -> Result<Self> {
        let len = file.size();
        let bloom_trailer_offset = len
            .checked_sub(std::mem::size_of::<u32>() as u64)
            .context("SST bloom-offset trailer is truncated")?;
        let raw_bloom_offset = file.read(bloom_trailer_offset, 4)?;
        let bloom_offset = u32::from_be_bytes([
            raw_bloom_offset[0],
            raw_bloom_offset[1],
            raw_bloom_offset[2],
            raw_bloom_offset[3],
        ]) as u64;
        ensure!(
            bloom_offset <= bloom_trailer_offset,
            "SST bloom offset is out of bounds"
        );
        let meta_trailer_offset = bloom_offset
            .checked_sub(std::mem::size_of::<u32>() as u64)
            .context("SST metadata-offset trailer is truncated")?;
        let raw_bloom = file.read(bloom_offset, bloom_trailer_offset - bloom_offset)?;
        let bloom_filter = Bloom::decode(&raw_bloom)?;
        let raw_meta_offset = file.read(meta_trailer_offset, 4)?;
        let block_meta_offset = u32::from_be_bytes([
            raw_meta_offset[0],
            raw_meta_offset[1],
            raw_meta_offset[2],
            raw_meta_offset[3],
        ]) as u64;
        ensure!(
            block_meta_offset <= meta_trailer_offset,
            "SST block-metadata offset is out of bounds"
        );
        let raw_meta = file.read(block_meta_offset, meta_trailer_offset - block_meta_offset)?;
        let block_meta = BlockMeta::decode_block_meta(&raw_meta[..])?;
        ensure!(!block_meta.is_empty(), "SST has no data blocks");
        ensure!(
            block_meta[0].offset == 0,
            "first SST block must start at zero"
        );
        ensure!(
            block_meta
                .windows(2)
                .all(|pair| pair[0].offset < pair[1].offset),
            "SST block offsets are not strictly increasing"
        );
        let block_meta_offset_usize =
            usize::try_from(block_meta_offset).context("SST metadata offset is too large")?;
        ensure!(
            block_meta.iter().all(|meta| {
                meta.offset
                    .checked_add(std::mem::size_of::<u32>())
                    .is_some_and(|end| end < block_meta_offset_usize)
            }),
            "SST block offset or checksum is out of bounds"
        );
        Ok(Self {
            file,
            first_key: block_meta[0].first_key.clone(),
            last_key: block_meta[block_meta.len() - 1].last_key.clone(),
            block_meta,
            block_meta_offset: block_meta_offset_usize,
            id,
            block_cache,
            bloom: Some(bloom_filter),
            max_ts: 0,
        })
    }

    /// 创建一个只含第一个键和最后一个键元数据的模拟 SST
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
        let offset = self
            .block_meta
            .get(block_idx)
            .context("SST block index is out of bounds")?
            .offset;
        let offset_end = self
            .block_meta
            .get(block_idx + 1)
            .map_or(self.block_meta_offset, |x| x.offset);
        let range_len = offset_end
            .checked_sub(offset)
            .context("SST block offsets are reversed")?;
        let block_len = range_len
            .checked_sub(std::mem::size_of::<u32>())
            .context("SST block checksum is truncated")?;
        let block_data_with_chksum: Vec<u8> = self.file.read(offset as u64, range_len as u64)?;
        let block_data = &block_data_with_chksum[..block_len];
        let checksum = u32::from_be_bytes([
            block_data_with_chksum[block_len],
            block_data_with_chksum[block_len + 1],
            block_data_with_chksum[block_len + 2],
            block_data_with_chksum[block_len + 3],
        ]);
        if checksum != crc32fast::hash(block_data) {
            bail!("block checksum mismatched");
        }
        Ok(Arc::new(Block::decode_checked(block_data)?))
    }

    /// 从磁盘读取一个块，经过块缓存。
    pub fn read_block_cached(&self, block_idx: usize) -> Result<Arc<Block>> {
        if let Some(ref block_cache) = self.block_cache {
            let blk = block_cache
                .try_get_with((self.id, block_idx), || self.read_block(block_idx))
                .map_err(|e| anyhow!("{}", e))?;
            Ok(blk)
        } else {
            self.read_block(block_idx)
        }
    }

    /// 找到可能包含 `key` 的块。
    pub fn find_block_idx(&self, key: KeySlice) -> usize {
        self.block_meta
            .partition_point(|meta| meta.first_key.as_key_slice() <= key)
            .saturating_sub(1)
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
