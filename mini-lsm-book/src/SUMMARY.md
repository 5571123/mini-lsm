<!--
  mini-lsm-book © 2022-2026 by Alex Chi Z is licensed under CC BY-NC-SA 4.0
-->

# Mini-LSM —— 用 Rust 构建数据库存储引擎

[学习数据库内部原理](./00-preface.md)
[Mini-LSM 概览](./00-overview.md)
[环境配置](./00-get-started.md)

- [第一周概览：Mini-LSM](./week1-overview.md)
  - [内存表（memtable）](./week1-01-memtable.md)
  - [归并迭代器](./week1-02-merge-iterator.md)
  - [块（block）](./week1-03-block.md)
  - [Sorted String Table（SST）](./week1-04-sst.md)
  - [读路径](./week1-05-read-path.md)
  - [写路径](./week1-06-write-path.md)
  - [加餐时间：SST 优化](./week1-07-sst-optimizations.md)

- [第二周概览：压缩（compaction）与持久化](./week2-overview.md)
  - [压缩实现](./week2-01-compaction.md)
  - [简单压缩策略](./week2-02-simple.md)
  - [Tiered 压缩策略](./week2-03-tiered.md)
  - [Leveled 压缩策略](./week2-04-leveled.md)
  - [清单（manifest）](./week2-05-manifest.md)
  - [预写日志（WAL）](./week2-06-wal.md)
  - [加餐时间：批量写入与校验和](./week2-07-snacks.md)

- [第三周概览：MVCC](./week3-overview.md)
  - [时间戳编码与重构](./week3-01-ts-key-refactor.md)
  - [快照：内存表与时间戳](./week3-02-snapshot-read-part-1.md)
  - [快照：事务 API](./week3-03-snapshot-read-part-2.md)
  - [水位线与 GC](./week3-04-watermark.md)
  - [事务工作区与原子提交](./week3-05-txn-occ.md)
  - [可串行化校验](./week3-06-serializable.md)
  - [加餐时间：压缩过滤器](./week3-07-compaction-filter.md)
- [你的余生](./week4-overview.md)

- [使用编码智能体的 Mini-LSM](./agent-fast-forward-overview.md)
  - [第 1 天：构建存储引擎](./week1-fast-forward.md)
  - [第 2 天：压缩与恢复](./week2-fast-forward.md)
  - [第 3 天：事务与 MVCC](./week3-fast-forward.md)
- [由 Raft.build 赞助](./sponsor.md)
