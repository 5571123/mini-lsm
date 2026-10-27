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

// Copyright 2021 TiKV Project Authors. Licensed under Apache-2.0.

use anyhow::{Result, bail, ensure};
use bytes::{BufMut, Bytes, BytesMut};

/// 实现一个布隆过滤器
pub struct Bloom {
    /// 过滤器的位数据
    pub(crate) filter: Bytes,
    /// 哈希函数的个数
    pub(crate) k: u8,
}

pub trait BitSlice {
    fn get_bit(&self, idx: usize) -> bool;
    fn bit_len(&self) -> usize;
}

pub trait BitSliceMut {
    fn set_bit(&mut self, idx: usize, val: bool);
}

impl<T: AsRef<[u8]>> BitSlice for T {
    fn get_bit(&self, idx: usize) -> bool {
        let pos = idx / 8;
        let offset = idx % 8;
        (self.as_ref()[pos] & (1 << offset)) != 0
    }

    fn bit_len(&self) -> usize {
        self.as_ref().len() * 8
    }
}

impl<T: AsMut<[u8]>> BitSliceMut for T {
    fn set_bit(&mut self, idx: usize, val: bool) {
        let pos = idx / 8;
        let offset = idx % 8;
        if val {
            self.as_mut()[pos] |= 1 << offset;
        } else {
            self.as_mut()[pos] &= !(1 << offset);
        }
    }
}

impl Bloom {
    /// 解码一个布隆过滤器
    pub fn decode(buf: &[u8]) -> Result<Self> {
        ensure!(buf.len() >= 5, "bloom filter is truncated");
        let checksum = u32::from_be_bytes([
            buf[buf.len() - 4],
            buf[buf.len() - 3],
            buf[buf.len() - 2],
            buf[buf.len() - 1],
        ]);
        if checksum != crc32fast::hash(&buf[..buf.len() - 4]) {
            bail!("checksum mismatched for bloom filters");
        }
        let filter = &buf[..buf.len() - 5];
        ensure!(!filter.is_empty(), "bloom filter has no bits");
        let k = buf[buf.len() - 5];
        Ok(Self {
            filter: filter.to_vec().into(),
            k,
        })
    }

    /// 编码一个布隆过滤器
    pub fn encode(&self, buf: &mut Vec<u8>) {
        let offset = buf.len();
        buf.extend(&self.filter);
        buf.put_u8(self.k);
        let checksum = crc32fast::hash(&buf[offset..]);
        buf.put_u32(checksum);
    }

    /// 由条目数与 FPR 计算每个键的布隆过滤器位数
    pub fn bloom_bits_per_key(entries: usize, false_positive_rate: f64) -> usize {
        let size = -(entries as f64) * false_positive_rate.ln() / std::f64::consts::LN_2.powi(2);
        let locs = (size / (entries as f64)).ceil();
        locs as usize
    }

    /// 由键的哈希值构建布隆过滤器
    pub fn build_from_key_hashes(keys: &[u32], bits_per_key: usize) -> Self {
        let k = (bits_per_key as f64 * 0.69) as u32;
        let k = k.clamp(1, 30);
        let nbits = (keys.len() * bits_per_key).max(64);
        let nbytes = nbits.div_ceil(8);
        let nbits = nbytes * 8;
        let mut filter = BytesMut::with_capacity(nbytes);
        filter.resize(nbytes, 0);
        for h in keys {
            let mut h = *h;
            let delta = h.rotate_left(15);
            for _ in 0..k {
                let bit_pos = (h as usize) % nbits;
                filter.set_bit(bit_pos, true);
                h = h.wrapping_add(delta);
            }
        }
        Self {
            filter: filter.freeze(),
            k: k as u8,
        }
    }

    /// 检查布隆过滤器是否可能包含某个数据
    pub fn may_contain(&self, mut h: u32) -> bool {
        if self.k > 30 {
            // 短布隆过滤器可能采用的新编码
            true
        } else {
            let nbits = self.filter.bit_len();
            let delta = h.rotate_left(15);
            for _ in 0..self.k {
                let bit_pos = h % (nbits as u32);
                if !self.filter.get_bit(bit_pos as usize) {
                    return false;
                }
                h = h.wrapping_add(delta);
            }
            true
        }
    }
}
