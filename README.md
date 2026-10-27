![Mini-LSM: 用 Rust 构建数据库存储引擎](./mini-lsm-book/src/mini-lsm-banner.svg)

# Mini-LSM 中文版：用 Rust 构建数据库存储引擎

> **本仓库定位**：fork 自 [skyzh/mini-lsm](https://github.com/skyzh/mini-lsm)，作为课程**阶段 Ⅰ** 的项目底座。
> 小组在此基础上增加鉴权、简单的 WAL 加密等上层模块，**不改动底层 LSM 核心**。
> 相对上游已完成：教程与代码注释全量中文化、Windows 平台编译与测试修复、符号链接展开。

Mini-LSM 是一门面向系统与后端方向的数据库内核实践课：从内存表（memtable）和 SSTable 出发，
一路做到压缩（compaction）、崩溃恢复、多版本并发控制（MVCC）与事务。三周之后你会得到一个
可以真正跑起来的 LSM-Tree 存储引擎。

- 中文教程（mdBook 源文件）：[mini-lsm-book/src](./mini-lsm-book/src/SUMMARY.md)
- 翻译与代码风格约定：[docs/translation-guide.md](./docs/translation-guide.md)
- 阶段 Ⅰ 扩展指南（鉴权 + WAL 加密）：[docs/阶段一扩展指南.md](./docs/阶段一扩展指南.md)

## 三周之后你的引擎具备什么

* 一个有序的内存写入缓冲（可写内存表 + 只读内存表）与不可变 SSTable 文件；
* 跨越内存与磁盘的归并视图，支持点查与范围扫描；
* 多种压缩策略（simple leveled / tiered / leveled）与后台维护线程；
* 清单（manifest）与预写日志（WAL），可从崩溃中恢复；
* 快照、MVCC 垃圾回收、乐观并发控制，以及可串行化校验。

Mini-LSM 只覆盖数据库的存储层，不涉及 SQL 解析、查询优化、复制与分布式共识。

## 仓库结构

| 目录 | 内容 |
| --- | --- |
| `mini-lsm-starter` | **学生改这里**：留空待实现的骨架代码 |
| `mini-lsm` | 第 1～2 周的参考实现（含前两周测试用例） |
| `mini-lsm-mvcc` | 第 3 周 MVCC 的参考实现（含第三周测试用例） |
| `mini-lsm-book` | 课程正文（mdBook），已中文化 |
| `xtask` | `cargo x` 子命令：拷测试、跑检查、构建教材 |
| `scripts` | Windows 下的共享文件同步脚本 |
| `docs` | 翻译规范、阶段 Ⅰ 扩展指南 |

## 快速开始

### 环境要求

* Rust stable，`rustc` ≥ 1.97.1（工作区使用 edition 2024）；
* Linux / macOS / **Windows** 均可，本分支已在 Windows（MSVC 工具链）上验证编译与全部参考实现测试。

### 学生：在 `mini-lsm-starter` 里写代码

```bash
cargo x install-tools                  # 安装 cargo-nextest、mdbook（首次运行需要，可选）
cargo x copy-test --week 1 --day 1     # 把第一周第 1 天的测试用例拷进 starter
cargo x scheck                         # 在 starter 里跑 fmt + check + test + clippy
cargo run --bin mini-lsm-cli -- --path ./my.db --compaction leveled --enable-wal
```

`mini-lsm-cli` 是一个 REPL，支持 `put/get/delete/scan/flush/close` 等命令，参数：

| 参数 | 默认值 | 说明 |
| --- | --- | --- |
| `--path` | `lsm.db` | 数据库目录 |
| `--compaction` | `leveled` | `simple` / `leveled` / `tiered` / `none` |
| `--enable-wal` | 关闭 | 打开预写日志（第 2 周第 6 天后才有意义） |
| `--serializable` | 关闭 | 打开可串行化校验（第 3 周第 6 天后才有意义） |

压缩策略实验台（在内存里模拟压缩效果，不需要真的写盘）：

```bash
cargo run --bin compaction-simulator-ref
```

### 对答案：跑参考实现

```bash
cargo test -p mini-lsm          # 第 1～2 周参考实现，72 个测试
cargo test -p mini-lsm-mvcc     # 第 3 周 MVCC 参考实现，96 个测试
cargo run --bin mini-lsm-cli-ref
cargo run --bin mini-lsm-cli-mvcc-ref
```

## 课程目录（中文）

| 周 + 章节 | 主题 |
| --- | --- |
| 1.1 | 内存表（[week1-01](./mini-lsm-book/src/week1-01-memtable.md)） |
| 1.2 | 归并迭代器（[week1-02](./mini-lsm-book/src/week1-02-merge-iterator.md)） |
| 1.3 | 块（[week1-03](./mini-lsm-book/src/week1-03-block.md)） |
| 1.4 | SSTable（[week1-04](./mini-lsm-book/src/week1-04-sst.md)） |
| 1.5 | 读路径（[week1-05](./mini-lsm-book/src/week1-05-read-path.md)） |
| 1.6 | 写路径（[week1-06](./mini-lsm-book/src/week1-06-write-path.md)） |
| 1.7 | SST 优化：前缀键编码 + 布隆过滤器（[week1-07](./mini-lsm-book/src/week1-07-sst-optimizations.md)） |
| 2.1 | 压缩实现（[week2-01](./mini-lsm-book/src/week2-01-compaction.md)） |
| 2.2 | 简单分层压缩（[week2-02](./mini-lsm-book/src/week2-02-simple.md)） |
| 2.3 | 分层（tiered）压缩（[week2-03](./mini-lsm-book/src/week2-03-tiered.md)） |
| 2.4 | 分层（leveled）压缩（[week2-04](./mini-lsm-book/src/week2-04-leveled.md)） |
| 2.5 | 清单（manifest）（[week2-05](./mini-lsm-book/src/week2-05-manifest.md)） |
| 2.6 | 预写日志（WAL）（[week2-06](./mini-lsm-book/src/week2-06-wal.md)） |
| 2.7 | 批量写与校验和（[week2-07](./mini-lsm-book/src/week2-07-snacks.md)） |
| 3.1 | 时间戳键编码与重构（[week3-01](./mini-lsm-book/src/week3-01-ts-key-refactor.md)） |
| 3.2 | 快照读：内存表与时间戳（[week3-02](./mini-lsm-book/src/week3-02-snapshot-read-part-1.md)） |
| 3.3 | 快照读：事务 API（[week3-03](./mini-lsm-book/src/week3-03-snapshot-read-part-2.md)） |
| 3.4 | 水位线与垃圾回收（[week3-04](./mini-lsm-book/src/week3-04-watermark.md)） |
| 3.5 | 事务工作区与原子提交（[week3-05](./mini-lsm-book/src/week3-05-txn-occ.md)） |
| 3.6 | 可串行化校验（[week3-06](./mini-lsm-book/src/week3-06-serializable.md)） |
| 3.7 | 压缩过滤器（[week3-07](./mini-lsm-book/src/week3-07-compaction-filter.md)） |

构建本地教材（需要 `mdbook`）：

```bash
cargo x book        # 本地起服务预览
```

## 相对上游做了哪些改动

| 类别 | 说明 |
| --- | --- |
| 中文化 | `mini-lsm-book/src` 全部章节、三个 crate 的代码注释、`README` 均已译为中文；规则见 [docs/translation-guide.md](./docs/translation-guide.md) |
| Windows 兼容 | 见下表，上游代码在 Windows 上**无法编译、也无法通过测试** |
| 符号链接 | 上游用 git 符号链接在三个 crate 间共享源文件，Windows 下会退化成「一行路径」的文本文件，本仓库已展开为真实文件，并用 `scripts/sync_shared_files.py` 保持同步 |
| 文档 | 新增 `docs/阶段一扩展指南.md`（阶段 Ⅰ 的挂载点设计） |

Windows 兼容修复的具体位置：

| 文件 | 问题 | 处理 |
| --- | --- | --- |
| `mini-lsm{,-mvcc,-starter}/src/table.rs` | `std::os::unix::fs::FileExt::read_exact_at` 在 Windows 不存在 | 抽出 `read_exact_at` 辅助函数：Unix 用 `pread`，Windows 用 `seek_read` 循环补齐 |
| 同上（`FileObject::create`） | `File::open` 是只读句柄，`sync_all` 在 Windows 报「拒绝访问」 | 改为读写方式打开后再 `sync_all` |
| `mini-lsm{,-mvcc}/src/wal.rs`、`manifest.rs` | `O_APPEND` 打开后 `set_len` 缺少 `FILE_WRITE_DATA` 权限，无法截断损坏尾部 | Unix 保留 `append`；Windows 用读写方式打开，读完后 `seek(End(0))` 恢复追加语义 |
| `mini-lsm{,-mvcc}/src/lsm_storage.rs` | 对目录 `fsync`（`File::open(dir)`）在 Windows 报错 | Unix 保留；Windows 退化为空操作，并在注释中说明理由 |
| `mini-lsm/src/tests/harness.rs` | `MetadataExt::size()` 是 Unix 扩展 | 改用跨平台的 `Metadata::len()` |
| `xtask/src/main.rs` | `copy-test` 调用外部 `cp`，Windows 默认 PATH 没有 | 改用 `std::fs::copy` |

## 阶段 Ⅰ 的扩展方向

本项目的目标是**在 LSM 核心之上**做加法，两条主线：

1. **鉴权层**：在 `MiniLsm` 外面包一层 `AuthEngine`，用前缀键实现用户命名空间隔离，
   权限校验在进入 `put/get/delete/scan` 之前完成；
2. **WAL 加密**：只改 `wal.rs` 的编解码边界，把明文记录换成「随机 nonce + AEAD 密文」，
   SST、manifest 与压缩流程完全不动。

具体的挂载点、需要改哪几个函数、有哪些坑（校验和位置、崩溃恢复的尾部截断、密钥管理），
见 [docs/阶段一扩展指南.md](./docs/阶段一扩展指南.md)。

## 小组协作建议

* 第 1 周：两人分 `mem_table` / `iterators`，一人做 `block` + `table`，一人提前读第 2 章；
  周末统一跑 `cargo x copy-test --week 1` 全量测试。
* 第 2 周：压缩策略与 manifest/WAL 两条线并行，注意 `lsm_storage.rs` 是共同热点，改前先在群里认领。
* 第 3 周：MVCC 建议单人主力，其他人做鉴权层与文档。
* 合并前跑：`cargo check --workspace --all-targets`、`cargo test -p mini-lsm`、`cargo test -p mini-lsm-mvcc`。
* 改了共享文件（`mini-lsm-starter/src/debug.rs`、`mini-lsm/src/tests/*` 等）后，记得
  `python scripts/sync_shared_files.py`，否则另外两个 crate 会编译失败。

## 常见问题

**Q：`mini-lsm-starter` 里到处是 `unimplemented!()`，编译却通过了？**
这是刻意设计的：骨架能编译、测试会失败，你按章节逐个实现即可。真正跑测试前先
`cargo x copy-test --week N --day M`。

**Q：`mini-lsm-starter/src/tests` 是空的？**
测试用例不预置，由 `cargo x copy-test` 按需从参考实现拷入，避免学生看到答案。

**Q：注释里中英文混排？**
专有名词（SSTable、WAL、memtable、`Week 1 Day 1` 之类的课程定位标记）按翻译规范保留英文，
便于对照上游与本仓库的 diff。

**Q：重新 clone 上游后出一堆内容是路径的 .rs 文件？**
那是 git 符号链接在 Windows 下的退化结果。要么在开发者模式下 `git clone -c core.symlinks=true`，
要么运行 `python scripts/sync_shared_files.py`。

## 许可与致谢

* 代码（`mini-lsm-starter`、参考实现）：[Apache 2.0](./LICENSE)；
* 课程材料（`mini-lsm-book` 的 markdown 与插图）：版权归原作者 Alex Chi Z 所有，采用
  CC BY-NC-SA 4.0，中译本同样遵循该协议，**不得用于商业用途**；
* 原仓库：<https://github.com/skyzh/mini-lsm>，在线教程：<https://skyzh.github.io/mini-lsm/>。
