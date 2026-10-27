<!--
  mini-lsm-book © 2022-2026 by Alex Chi Z is licensed under CC BY-NC-SA 4.0
-->

# 内存表（memtable）

![本章概览](./lsm-tutorial/week1-01-overview.svg)

学完本章，你将能够：

* 基于跳表（skiplist）实现内存表。
* 实现冻结内存表的逻辑。
* 实现 LSM `get` 读路径中处理内存表的部分。
* 解释快照顺序、墓碑（tombstone）和两把状态锁如何在并发写入时保证点查正确。

把测试用例复制到起始代码并运行：

```
cargo x copy-test --week 1 --day 1
cargo x scheck
```

## 开始之前

起始引擎暴露了 `get`、`put` 和 `delete`，但还没有可用的存储结构。本章引入它的第一个数据来源：一个可写内存表，后面跟着零个或多个等待落盘的只读内存表。

实现任务时，请记住这些不变量：

1. 引擎刚好有一个当前可写内存表。
2. `imm_memtables` 从新到旧排列。
3. 从新往旧探查时，找到的某个键的第一个版本决定结果。空值就是墓碑，因此该键被判为不存在，搜索必须停止，不能继续查找更旧的值。
4. 替换可写内存表这类结构性变更是串行执行的，尽管同一个内存表内的读写可以并发进行。

> **写代码前先预测：** 假设可写内存表包含 `a -> delete`，最新的只读内存表包含 `a -> 2`，最旧的包含 `a -> 1`。`get(a)` 应返回什么？如果按相反顺序探查内存表，你会得到什么结果？

## 任务 1：跳表内存表

本任务需要修改：

```
src/mem_table.rs
```

先实现 LSM 存储引擎的内存结构：内存表。我们使用 [crossbeam-skiplist](https://docs.rs/crossbeam-skiplist/latest/crossbeam_skiplist/)，因为它支持无锁的并发读写。我们不会深入讲跳表。本课程中，你可以把跳表看作一个有序键值映射，它高效地支持并发访问。

`crossbeam-skiplist` 提供的方法与 Rust 标准库 `BTreeMap` 上的方法类似，包括 `insert`、`get` 和 `iter`。重要区别在于，`insert` 这类修改方法只需要对跳表的不可变引用，而不是可变引用。因此你的内存表实现不需要额外的互斥锁。

你也会注意到 `MemTable` 没有 `delete` 方法。在 Mini-LSM 中，键关联空值就表示删除。

本任务实现 `MemTable::create`、`MemTable::get` 和 `MemTable::put`。第一天的代码用 `MemTable::create` 初始化普通的内存内内存表。`put` 方法应覆盖同一键上已有的条目，因此单个内存表不会为一个键保存多个条目。

我们用 `bytes` crate 在内存表中存储数据。`bytes::Bytes` 与 `Arc<[u8]>` 类似：克隆或切片一个 `Bytes` 值不会复制其底层数据，因此这两种操作开销都很小。相反，每次操作只是创建指向同一份存储的又一个引用，当引用全部消失时这块存储被释放。

## 任务 2：引擎中的单个内存表

本任务需要修改：

```
src/lsm_storage.rs
```

现在把内存表加入 LSM 状态。`LsmStorageState::create` 初始化 0 号内存表，也就是初始的**可写内存表**。任意时刻，引擎只有一个可写内存表。内存表通常有大小上限——例如 256 MB——达到上限后就被冻结为只读内存表。

在 `lsm_storage.rs` 中，两个结构表示存储引擎：`MiniLsm` 和 `LsmStorageInner`。`MiniLsm` 是 `LsmStorageInner` 的薄封装。在第二周开始实现压缩（compaction）之前，大部分功能都加在 `LsmStorageInner` 上。

`LsmStorageState` 描述 LSM 存储引擎的当前结构。目前你只需要用到它的 `memtable` 字段，该字段保存当前可写内存表。实现 `LsmStorageInner::get`、`LsmStorageInner::put` 和 `LsmStorageInner::delete`，把每个请求直接转发给这个内存表。

![单个内存表的 LSM](./lsm-tutorial/week1-01-single.svg)

你的 `delete` 实现应为该键存储一个空切片。这个条目称为*删除墓碑*。`get` 实现应识别墓碑，并报告该键不存在。

访问内存表时，需要获取 `state` 锁。由于 `MemTable::put` 只需要不可变引用，即使写入内存表，你也只需要对 `state` 的读锁。这一设计允许多个线程并发访问内存表。

## 任务 3：写路径——冻结内存表

第一天继续使用普通的 `MemTable::create` 构造函数。`MemTable::create_with_wal`、`MemTable::recover_from_wal` 和 `Wal` 的 API 属于第二周第六天，不属于第一天。下面的代码片段只预览了如何在状态锁之外创建带 WAL 的内存表；不要在第一天实现或照抄这项后续工作。

本任务需要修改：

```
src/lsm_storage.rs
src/mem_table.rs
```

![单个内存表的 LSM](./lsm-tutorial/week1-01-frozen.svg)

内存表不能无限增长，因此当它达到大小上限时，你必须冻结它——之后还要落盘到磁盘。`LsmStorageOptions::target_sst_size` 既是目标 SSTable 大小，也近似作为内存表容量。不要把它和 `num_memtable_limit` 混淆。这个容量是软上限，因此冻结是尽力而为的操作。

本任务中，每次放入或删除键时都要跟踪内存表的近似大小。你可以在每次调用 `put` 时把键和值的长度相加来估算。如果同一个键被写入两次，你可以把两次写入都计入，尽管跳表只保留最新的值。内存表达到上限后，调用 `force_freeze_memtable` 将它冻结，并创建一个新的可写内存表。

`LsmStorageInner` 中的 `state: Arc<RwLock<Arc<LsmStorageState>>>` 字段采用写时复制（copy-on-write，CoW）策略，安全且并发地管理 LSM 树的结构状态：

1. 内层 `Arc<LsmStorageState>`：它持有 `LsmStorageState` 在结构上**不可变的快照**，包含内存表列表和 SSTable 引用。克隆这个 `Arc` 开销很小——只是递增一个原子引用计数——并在一次操作期间为读线程提供一致的结构视图。

2. `RwLock<Arc<LsmStorageState>>`：这把读写锁保护指向当前快照的指针。
    * **读线程**获取读锁，克隆 `Arc<LsmStorageState>`，然后立即释放锁。之后它们基于自己的快照工作，不再持有全局状态锁。
    * **写线程**获取写锁，克隆底层 `LsmStorageState`，在克隆上应用结构变更，把它包进新的 `Arc`，然后替换当前快照。

3. 外层 `Arc<RwLock<...>>`：让多个线程安全地共享这把锁，并通过它访问和更新当前快照。

这种 CoW 方式让读线程以最少的阻塞拿到有效且一致的快照。写线程原子地替换整个结构快照，从而缩短临界区并提升并发度。

由于多个线程都可能写入存储引擎，它们可能并发调用 `force_freeze_memtable`。你必须防止这些调用之间发生竞争。

有几种操作会修改 LSM 状态：冻结可写内存表、把内存表落盘为 SSTable，以及执行垃圾回收或压缩（compaction）。这些操作可能涉及 I/O。一种直观的加锁策略是在写锁内完成整个状态变更：

```rust,no_run
fn freeze_memtable(&self) {
    let mut guard = self.state.write();
    let mut snapshot = guard.as_ref().clone();
    let old_memtable = std::mem::replace(
        &mut snapshot.memtable,
        Arc::new(MemTable::create(self.next_sst_id())),
    );
    snapshot.imm_memtables.insert(0, old_memtable);
    *guard = Arc::new(snapshot);
}
```

这个做法目前可行。不过，考虑为每个内存表创建一个预写日志（WAL）文件。预写日志（WAL）是只追加的写入记录，让引擎在崩溃后可以恢复——你会在第二周实现它。这里的要点是：创建这样一个文件属于 I/O 操作，可能耗时数毫秒：

```rust,no_run
fn freeze_memtable(&self) -> Result<()> {
    let mut guard = self.state.write();
    let id = self.next_sst_id();
    let memtable = Arc::new(MemTable::create_with_wal(
        id,
        self.path_of_wal(id),
    )?); // <- 可能耗时数毫秒。
    // 在这里克隆并更新结构快照。
    // ...
    Ok(())
}
```

内存表冻结期间，其他线程在数毫秒内都无法访问 LSM 状态，从而产生延迟尖峰。

为避免这个问题，把 I/O 放到临界区之外：

```rust,no_run
fn freeze_memtable(&self) -> Result<()> {
    let id = self.next_sst_id();
    let memtable = Arc::new(MemTable::create_with_wal(
        id,
        self.path_of_wal(id),
    )?); // <- 可能耗时数毫秒。
    {
        let mut guard = self.state.write();
        let mut snapshot = guard.as_ref().clone();
        let old_memtable = std::mem::replace(&mut snapshot.memtable, memtable);
        snapshot.imm_memtables.insert(0, old_memtable);
        *guard = Arc::new(snapshot);
    }
    Ok(())
}
```

现在状态写锁内不再有高开销操作。接下来，假设一个内存表即将达到容量上限，两个线程各自添加一个键。两个线程都观察到内存表已达到容量，并决定冻结它。如果没有额外的同步，其中一个线程可能在另一个线程安装新内存表之后，立刻把这个新建的空内存表冻结掉。

为防止这种竞争，用 `state_lock` 串行化所有状态修改，并在获取它之后重新检查条件：

```rust,no_run
fn put(&self, key: &[u8], value: &[u8]) {
    // 写入内存表，检查其容量，然后释放状态读锁。
    if memtable_reaches_capacity_on_put {
        let state_lock = self.state_lock.lock();
        if /* 当前内存表仍然超过其容量 */ {
            self.freeze_memtable(&state_lock)?;
        }
    }
}
```

后续章节会经常见到这个模式。例如，L0 落盘遵循这样的流程：

```rust,no_run
fn force_flush_next_imm_memtable(&self) {
    let state_lock = self.state_lock.lock();
    // 取出最旧的内存表，然后释放状态读锁。
    // 把内容写入磁盘。
    // 获取状态写锁并安装更新后的快照。
}
```

这种方式保证同一时刻只有一个线程修改 LSM 状态，同时仍允许并发访问存储引擎。

修改 `put` 和 `delete`，让它们遵守内存表的软容量上限。内存表达到上限时，调用 `force_freeze_memtable`。测试集不覆盖这种并发场景，因此请仔细考虑可能出现的竞争。让每个临界区尽可能小。

用 `self.next_sst_id()` 分配下一个内存表 ID。`imm_memtables` 向量从新到旧保存内存表，因此 `imm_memtables.first()` 是最近冻结的内存表。

## 任务 4：读路径——Get

本任务需要修改：

```
src/lsm_storage.rs
```

现在你有多个内存表，请更新读路径的 `get` 方法，让它取出键的最新版本。从新到旧探查内存表。

## 本章检查点

你的引擎现在应支持在一个可写内存表和多个只读内存表上执行 `put`、`delete` 和 `get`。冻结一个已满的内存表时，它不应在高开销工作期间阻塞无关的状态访问，也不应立即冻结替换用的新内存表。

除了通过测试，还要确认你能找到本章开头每条不变量对应的代码。为一个被覆盖的键构造两内存表的例子，再为墓碑构造一个例子，然后确认把探查顺序颠倒会产生错误结果。

## 检验你的理解

结合你的实现回答正确性问题。对于涉及可能竞争或错误结果的问题，给出具体的执行过程或输入，而不只是笼统的解释。

### 正确性与并发

* 内存表为什么不提供 `delete` API？
* 内存表保存每一次写入、而不是只保存键的最新版本，这样做合理吗？例如，假设用户向同一个内存表依次写入 `a -> 1`、`a -> 2` 和 `a -> 3`。
* 为什么需要同时使用 `state` 和 `state_lock`？能不能只用 `state.read()` 和 `state.write()`？
* 构造最小的例子，使按错误顺序探查内存表时返回过期值；再构造一个例子，使已删除的值重新出现。
* 在若干地方，你可能会先获取状态读锁、释放它，再获取写锁。这两个操作可能发生在相互调用的不同函数中。这与直接把读锁升级为写锁有何不同？升级是否必要，代价是什么？

### 性能与设计

* LSM 树能否用其他数据结构实现内存表？跳表有哪些优缺点？
* 内存表的内存布局高效吗？数据局部性好吗？考虑 `Bytes` 的实现方式以及它在跳表中的存储方式。你可以怎样优化内存表的布局？
* **文档阅读：** 阅读 `parking_lot` 的 [`RwLock` 公平性一节](https://docs.rs/parking_lot/latest/parking_lot/type.RwLock.html#fairness)。当写线程已经在等待当前读线程释放锁时，等待获取锁的读线程可能会遇到什么情况？最终公平（eventual fairness）与严格的先进先出服务有何不同？

用[第一周周末自检](./week1-overview.md#end-of-week-self-check)校验核心不变量。其余设计问题可能有多个站得住脚的答案；在比较取舍之前，先说明你的工作负载和假设。你也可以在 Discord 社区讨论。

## 附加任务

* **更多内存表格式。** 实现其他内存表格式，例如 B 树、向量或自适应基数树（ART）内存表。

{{#include copyright.md}}
