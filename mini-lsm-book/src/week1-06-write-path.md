<!--
  mini-lsm-book © 2022-2026 by Alex Chi Z is licensed under CC BY-NC-SA 4.0
-->

# 写路径

![本章概览](./lsm-tutorial/week1-05-overview.svg)

学完本章，你将能够：

* 实现带 L0 落盘的 LSM 写路径。
* 实现正确更新 LSM 状态的逻辑。
* 解释让已落盘的 SSTable 可见、同时不丢失也不重复只读内存表的状态转换。
* 使用键范围元数据跳过 SSTable，而不改变读取结果。


把测试用例复制到起始代码并运行：

```
cargo x copy-test --week 1 --day 6
cargo x scheck
```

## 开始之前

读路径已经可以消费 SSTable，但第五天的测试是直接创建这些 SSTable 的。本章补完写路径：引擎自己把最旧的只读内存表转成 SSTable，并原子地把它装入 L0。

重要的不变量是：

1. 下一次落盘只选择最旧的只读内存表。
2. `state_lock` 把落盘与其他结构变更串行化。
3. 构建和写入 SSTable 在 `state` 读写锁之外进行。
4. 安装 SSTable 时，在同一次快照更新中精确移除被落盘的那个内存表、把 `SsTable` 插入 `sstables`，并把它的 ID 插入 `l0_sstables` 的前端——也就是最新的一侧。
5. SSTable 过滤只能在该文件的键范围无法为请求做出贡献时才排除它。优化不得改变查询结果。

> **写代码前先预测：** 假设 `imm_memtables` 从新到旧包含 ID `[7, 6, 5]`，`l0_sstables` 包含 `[4, 3]`。写出一次正确落盘之后这两个向量。哪些断言能发现落盘了错误的内存表，或安装了错误的 SSTable ID？

## 任务 1：把内存表落盘为 SSTable

此时内存结构和磁盘结构都已就绪，存储引擎可以从它们读取并合并数据。接下来你实现落盘——把数据从内存移到磁盘的过程——并完成 Mini-LSM 的第一周。

本任务需要修改：

```
src/lsm_storage.rs
src/mem_table.rs
```

修改 `LsmStorageInner::force_flush_next_imm_memtable` 和 `MemTable::flush`。在 `LsmStorageInner::open` 中，如果数据库目录不存在就创建它。落盘一个内存表需要三步：

* 选择一个要落盘的内存表。
* 用该内存表创建 SSTable 文件。
* 从只读内存表列表中移除该内存表，并把新的 SSTable 加入 L0。

我们还没有讨论第 0 层（L0）。它包含由内存表落盘直接产生的 SSTable 文件，因此它们的键范围可能重叠。第一周期间，所有磁盘上的 SSTable 都留在 L0。第二周你将探索分层（leveled）和分级（tiered）压缩策略如何高效地组织 SSTable。

创建 SSTable 计算开销大，并且涉及 I/O。不要在整个过程中持有 `state` 读写锁：那样会阻塞其他操作并造成较大的延迟尖峰。`state_lock` 互斥锁用于串行化修改 LSM 树状态的操作。请谨慎使用这两把锁，既避免竞争，又保持临界区简短。

调用方可能在获取 `state_lock` 之前观察到只读内存表列表非空，随后却发现另一个落盘已经先把列表清空了。请在持有 `state_lock` 时重新检查列表，如果它已经为空就直接成功返回。

测试集没有覆盖所有并发情况，因此请仔细推演同步逻辑。`imm_memtables` 中最后一个内存表是最旧的，因此也是要落盘的那个。

<details>

<summary>剧透：L0 落盘伪代码</summary>

```rust,no_run
fn force_flush_next_imm_memtable(&self) -> Result<()> {
    let _state_lock = self.state_lock.lock();

    let memtable_to_flush = {
        let guard = self.state.read();
        let Some(memtable) = guard.imm_memtables.last() else {
            return Ok(());
        };
        memtable.clone()
    };

    let sst_id = memtable_to_flush.id();
    let sst = build_sst_from_memtable(&memtable_to_flush, sst_id)?;

    {
        let mut guard = self.state.write();
        let mut snapshot = guard.as_ref().clone();
        let removed = snapshot.imm_memtables.pop().unwrap();
        assert_eq!(removed.id(), sst_id);
        snapshot.l0_sstables.insert(0, sst_id);
        snapshot.sstables.insert(sst_id, sst);
        *guard = Arc::new(snapshot);
    }

    Ok(())
}
```

</details>

## 任务 2：触发落盘

本任务需要修改：

```
src/lsm_storage.rs
src/compact.rs
```

当只读内存表的数量达到存储配置中的 `num_memtable_limit` 时，把最旧的一个落盘。这一工作由后台落盘线程执行。`MiniLsm` 封装中已经包含启动线程并通知它停止的代码。

在 `compact.rs` 中实现 `LsmStorageInner::trigger_flush`，在 `lsm_storage.rs` 中实现 `MiniLsm::close`。后台线程每 50 毫秒调用一次 `trigger_flush`。当内存表数量达到上限时，调用 `force_flush_next_imm_memtable`。用户调用 `close` 时，通知后台线程停止，并等待落盘线程——第二周还要等待压缩线程——结束。

## 任务 3：过滤 SSTable

现在你有了可用的存储引擎，用 `mini-lsm-cli` 与它交互：

```shell
cargo run --bin mini-lsm-cli -- --compaction none
```

在提示符下运行：

```
fill 1000 3000
get 2333
flush
fill 1000 3000
get 2333
flush
get 2333
scan 2000 2333
```

如果你插入更多数据，可以观察后台线程在未显式执行 `flush` 命令时自动把内存表落盘到 L0。

最后，实现一个简单的 SSTable 过滤优化。利用请求的键或键范围，以及每个 SSTable 的首键和末键，排除无法产生任何结果的 SSTable。这样归并迭代器就不必读取这些文件。

本任务需要修改：

```
src/lsm_storage.rs
src/iterators/*
src/lsm_iterator.rs
```

更新读路径，跳过不可能包含所请求键、或不与所请求范围重叠的 SSTable。同时实现 `num_active_iterators`，让测试可以验证这项优化。对于 `MergeIterator` 和 `TwoMergeIterator`，返回其子迭代器活跃迭代器数量之和。如果你保留了起始代码中 `MergeIterator` 的字段，记得把 `MergeIterator::current` 也算进去。对于 `LsmIterator` 和 `FusedIterator`，把计数委托给内部迭代器。

你可以实现 `range_overlap` 和 `key_within` 这类辅助函数来简化代码。

## 本章检查点

Mini-LSM 现在应能自己创建 SSTable，在只读内存表数量达到上限时自动落盘，干净地关闭后台线程，并避免打开键范围不可能影响读取的 SSTable。

测试通过后，记录一次手动落盘前后的状态：可写内存表 ID、只读内存表 ID、L0 ID，以及 `sstables` 映射中的键。确认在整个转换过程中，每个键始终只有一份逻辑副本可见。然后在关闭 SSTable 过滤的情况下执行同样的读取，确认只有迭代器数量变化，结果不变。

## 检验你的理解

### 正确性与状态转换

* 如果用户请求删除同一个键两次，会发生什么？
* 状态更新为什么必须验证从 `imm_memtables` 中移除的内存表 ID，与构建 SSTable 时使用的 ID 相同？
* 构造一种交错执行：如果没有 `state_lock`，两次落盘选中同一个最旧内存表，状态会被破坏。
* 回顾第一天的冻结路径：你的 `put` 是否一直持有状态读锁直到内存表写入完成？如果它在保存内存表 `Arc` 之后就释放锁，这次写入是否可能发生在内存表被冻结之后？
* 对扫描边界包含、排除和无界的每种组合，说明在什么条件下可以安全地排除一个 SSTable 范围。

### 内存与性能

* 迭代器初始化时，同时加载了多少内存或多少个块？在一次扫描期间测量 `num_active_iterators`，并解释它为什么会变化。

### 生产设计

* 假设用户想 *fork* 一棵 LSM 树：在导入数据之后，他们创建两份完全相同的数据集并独立修改。一个简单但低效的实现是把每个 SSTable 和内存结构都复制到新目录。由于磁盘上的 SSTable 不可变，fork 可以改为复用父级的文件。在不复制数据的前提下，你会如何高效实现？参见 [Neon Branching](https://neon.tech/docs/introduction/branching)。
* 设想一个多租户 LSM 系统，在一台 128 GB 内存的机器上托管 10,000 个数据库。如果每个内存表的大小上限是 256 MB，所有内存表一共需要多少内存？
  * 你显然没有足够的内存让它们同时都达到这个上限。如果每个租户仍然各自拥有内存表，你会如何设计落盘策略以适配全局内存预算？让租户共用一个内存表——例如在每个键前缀中编码租户 ID——是否合理？

用[第一周周末自检](./week1-overview.md#end-of-week-self-check)校验核心不变量。其余设计问题可能有多个站得住脚的答案；在比较取舍之前，先说明你的工作负载和假设。你也可以在 Discord 社区讨论。

## 附加任务

* **实现写停顿/L0 停顿。** 当内存表数量远超上限时，暂停用户写入。在第二周实现压缩之后，你还可以根据 L0 SSTable 的数量添加写停顿。
* **前缀扫描。** 你可以实现前缀扫描接口并利用前缀信息，从而过滤更多 SSTable。

{{#include copyright.md}}
