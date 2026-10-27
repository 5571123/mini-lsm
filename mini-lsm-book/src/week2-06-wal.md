<!--
  mini-lsm-book © 2022-2026 by Alex Chi Z is licensed under CC BY-NC-SA 4.0
-->

# 预写日志（WAL）

![本章概览](./lsm-tutorial/week2-06-overview.svg)

学完本章，你将能够：

* 把内存表的更新编码进预写日志（WAL），并同步它的缓冲数据。
* 在清单（manifest）中记录内存表的生命周期，并在重启后从 WAL 恢复它们。
* 恢复从新到旧的内存表顺序，以及一个不冲突的下一个 SST ID。
* 解释 `sync` 和 `close` 提供的确切持久化保证。

把测试用例复制到起始代码并运行：

```
cargo x copy-test --week 2 --day 6
cargo x scheck
```

## 开始之前

清单（manifest）让 SST 布局可以恢复，但它无法恢复进程崩溃时仍在内存中的写入。预写日志（WAL）会把每个内存表的内容持久化，直到该内存表被 SST 安全地表示出来。

请记住这些不变量：

1. 每条 WAL 记录恰好属于一个内存表；如果该内存表被落盘，它的 WAL ID 就成为 SST ID。
2. 清单（manifest）记录每个新的内存表，使恢复过程知道哪些 WAL 文件是有效的。落盘记录在文件被物理删除之前，先在逻辑上让对应的 WAL 退役。
3. 写入可能仍留在 `BufWriter` 和操作系统页缓存中。只有在 `flush()` 之后再调用 `sync_all()`，它们才变得持久。
4. 恢复出的只读内存表从新到旧排列，与正常执行期间创建的内存表一样。
5. `next_sst_id` 等于在 SST 状态或有效 WAL 状态中观察到的最大 ID 加一。

> **写代码前先预测：** 清单（manifest）中包含 `NewMemtable(7)` 但没有 `Flush(7)`，而 `00007.wal` 包含 `k -> v`。恢复应构建什么？如果 `Flush(7)` 已持久化，但旧的 WAL 文件在崩溃前没有被删除，恢复是否应重放它？

## 任务 1：WAL 编码

本任务需要修改：

```
src/wal.rs
```

上一章持久化了 LSM 结构，并在正常关闭时落盘每个内存表。崩溃可能绕过这条关闭路径。为了恢复未落盘的数据，把每次内存表更新记录到预写日志（WAL）中。只有当 `self.options.enable_wal` 为 true 时才启用 WAL。

WAL 的编码是一串键值对。

```
| key_len | key | value_len | value |
```

实现 `recover`，把 WAL 重放进一个内存表，并重新以追加方式打开该文件。

WAL 使用 `BufWriter` 来减少写路径上的系统调用次数。更新一个键本身并不保证该记录已到达持久存储。引擎在 `sync` 成功时给出这个保证。实现 `sync` 时，先调用 `flush()` 把字节从 `BufWriter` 移到文件中，再调用 `get_mut().sync_all()` 同步该文件。你不需要为每一次单独的写入调用 `sync_all()`。

## 任务 2：集成 WAL

本任务需要修改：

```
src/mem_table.rs
src/wal.rs
src/lsm_storage.rs
```

`MemTable` 有一个可选的 WAL。存在 WAL 时，把每次更新追加到该 WAL。如果 `enable_wal` 为 true，用 `create_with_wal` 创建每个内存表，并在让新的带 WAL 内存表可用于写入之前追加 `ManifestRecord::NewMemtable`。

把每个 WAL 存为数据库目录下的 `<memtable_id>.wal`。如果该内存表之后被落盘，就为它的 SST 复用这个 ID。

## 任务 3：从 WAL 恢复

本任务需要修改：

```
src/lsm_storage.rs
```

如果启用了 WAL，在打开数据库时从各自的 WAL 恢复有效内存表。同时实现数据库的 `sync` 方法。`sync` 成功返回之后，在该同步点之前完成的写入必须在重启后可恢复。在这个设计中，同步当前内存表的 WAL 就提供了这个保证，因为只读内存表在被替换时已经同步过。

```
cargo run --bin mini-lsm-cli -- --enable-wal
```

把 `next_sst_id` 恢复为 `max{memtable ID, SST ID} + 1`。启用 WAL 时，`close` 改为同步它们，而不是把每个内存表落盘为 SST。返回之前，停止并 join 压缩线程和落盘线程。

## 本章检查点

启用 WAL 时，已同步的写入即使所在的内存表从未落盘，也应在重启后存活。恢复过程应忽略被清单（manifest）落盘记录标记退役的过期 WAL 文件，按从新到旧的顺序重建有效的只读内存表，用新的 WAL 创建一个全新的可写内存表，并选择一个未使用的 ID。

测试不能只覆盖正常关闭：创建多个内存表、同步、重新打开，并跟踪标识每个有效 WAL 的清单（manifest）记录。解释如果进程在 `sync` 之前停止、在 `sync` 之后停止，以及在落盘记录持久化之后但它的 WAL 文件被删除之前停止，哪些写入保证能存活。

## 检验你的理解

### 恢复与持久性

* 你的引擎中应在何时调用 `fsync`？如果调用得太频繁（例如每次 put 键请求都调用）会怎样？
* **实验：** 在你自己的存储设备上测量批量 `fsync` 的延迟和吞吐。记录操作系统、文件系统、设备、队列深度和批大小；为什么不存在一个与硬件无关的统一答案？
* 你何时可以告诉用户他们的修改（put/delete）已经持久化？
* 为什么新的内存表必须在对其 WAL 的已同步写入被认为可恢复之前记录到清单（manifest）中？
* 为什么已落盘内存表的 WAL 只能在清单（manifest）的落盘记录持久化之后才删除？
* 给定 WAL ID 4 和 9，以及有效 SST ID 3、7 和 12，下一个内存表应使用哪个 ID？
* 你如何处理 WAL 中的损坏数据？

### 性能与设计

* 是否可能设计一个不带 WAL 的 LSM 引擎（也就是把 L0 当作 WAL）？这种设计会带来什么影响？

用[第二周周末自检](./week2-overview.md#end-of-week-self-check)校验核心不变量。其余设计问题可能有多个站得住脚的答案；在比较取舍之前，先说明你的工作负载和假设。你也可以在 Discord 社区讨论。

{{#include copyright.md}}
