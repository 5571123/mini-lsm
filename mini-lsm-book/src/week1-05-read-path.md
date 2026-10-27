<!--
  mini-lsm-book © 2022-2026 by Alex Chi Z is licensed under CC BY-NC-SA 4.0
-->

# 读路径

![本章概览](./lsm-tutorial/week1-05-overview.svg)

学完本章，你将能够：

* 把 SSTable 接入 LSM 读路径。
* 实现带 SSTable 的 LSM 读路径 `get`。
* 实现带 SSTable 的 LSM 读路径 `scan`。
* 追踪新旧程度、墓碑、范围边界和状态快照如何跨越内存与磁盘决定逻辑结果。

把测试用例复制到起始代码并运行：

```
cargo x copy-test --week 1 --day 5
cargo x scheck
```

## 开始之前

内存表和 SSTable 各自都能被独立查询。存储引擎现在必须把它们组合成一个逻辑视图，使用户看不到实现细节——有多少结构包含某个键，或者这些结构位于何处。

请保持这些读路径不变量：

1. 可写和只读内存表优先于 L0 SSTable，并且在每组内部，较新的来源优先于较旧的来源。
2. 较新来源中的墓碑会隐藏同一键的所有较旧值。
3. `scan` 在请求的精确边界内返回有序、唯一且存活的键。
4. 定位可能落在下一个更大的键上，因此 `get` 只有在确认键完全相等之后才返回值。
5. 创建和定位 SSTable 迭代器可能执行 I/O，因此应在释放 `state` 锁之后、使用克隆出的一致快照进行。

> **写代码前先预测：** 可写内存表包含 `b -> delete` 和 `d -> 4`；只读内存表包含 `a -> 1` 和 `b -> 2`；最新的 L0 SSTable 包含 `a -> 0`、`c -> 3` 和 `d -> 3`。`get(a)`、`get(b)` 以及包含 `a` 和 `d` 的扫描分别应返回什么？对每个结果，指出哪个来源胜出。

## 任务 1：双路归并迭代器

本任务需要修改：

```
src/iterators/two_merge_iterator.rs
```

你已经为同类型迭代器（例如内存表迭代器）实现了归并迭代器。现在 SSTable 格式已经实现，引擎同时有内存中的内存表和磁盘上的 SSTable。一次扫描必须把内存表迭代器和 SSTable 迭代器合并成一个流。为此，实现 `TwoMergeIterator<X, Y>`，它可以合并两种不同的迭代器类型。

因为只有两个迭代器，`TwoMergeIterator` 不需要二叉堆。用一个标志就能表示当前哪个迭代器优先。与 `MergeIterator` 一样，当两个迭代器包含相同的键时，第一个迭代器优先。

## 任务 2：读路径——Scan

本任务需要修改：

```
src/lsm_iterator.rs
src/lsm_storage.rs
```

实现 `TwoMergeIterator` 之后，我们可以把 `LsmIteratorInner` 改成下面的类型：

```rust,no_run
type LsmIteratorInner =
    TwoMergeIterator<MergeIterator<MemTableIterator>, MergeIterator<SsTableIterator>>;
```

这个类型把内存表和 SSTable 的数据组合成存储引擎的内部迭代器。

SSTable 迭代器不支持扫描的结束边界。请在 `LsmIterator` 中施加该边界，把它的构造函数改为接受 `end_bound`：

```rust,no_run
pub(crate) fn new(iter: LsmIteratorInner, end_bound: Bound<Bytes>) -> Result<Self> {}
```

然后更新迭代逻辑，按边界的语义停止：在排除的结束键之前停止，或在包含的结束键之后停止。

测试会创建由 `l0_sstables` 引用的内存表和 SSTable；你的扫描必须正确返回它们合并后的内容。下一章之前你不需要实现落盘。现在只需更新 `LsmStorageInner::scan`，为所有内存表和 L0 SSTable 创建归并迭代器，从而完成引擎的扫描路径。

创建并初次定位 `SsTableIterator` 可能执行 I/O，因此不要在持有 `state` 锁时进行。先获取读锁并克隆包含状态快照的 `Arc`，释放锁之后再为每个 L0 SSTable 创建迭代器，并合并得到的流。

```rust,no_run
fn scan(&self) {
    let snapshot = {
        let guard = self.state.read();
        Arc::clone(&guard)
    };
    // 创建迭代器并对它们定位
}
```

`l0_sstables` 向量只保存 SSTable ID。请从 `sstables` 映射中取出对应的 `SsTable` 对象。

## 任务 3：读路径——Get

本任务需要修改：

```
src/lsm_storage.rs
```

把 `get` 处理为：先在内存表中直接查找，如有必要，再在 SSTable 的归并迭代器上定位。定位可能落在所请求的键上，也可能落在下一个更大的键上，因此只有当迭代器的键与所请求的键完全匹配时才返回值。与扫描路径一样，尽量缩短 `state` 锁的临界区。保持从新到旧的优先级，并把空值当作墓碑，而不是继续读取更旧的数据。

## 本章检查点

引擎现在应能在所有可写、只读和磁盘结构上呈现一致的读视图。对一个出现在多个来源中的键、一个被较新来源删除的键，以及一个在所有来源中都不存在的键，分别验证点查和扫描。

覆盖扫描边界的包含与排除的四种组合，以及无界范围。对每个测试，在运行之前先预测返回的第一个和最后一个键。最后，检查 `state` 读守卫的生命周期，确认在持有该守卫期间没有创建任何 SSTable 迭代器。

## 检验你的理解

### 正确性

* 在上面的预测示例中，如果把 `TwoMergeIterator` 的两个输入互换，每个结果会如何变化？
* 构造最小的状态，使找到墓碑后继续搜索会让已删除的键重新出现。
* 对 `b` 的定位落在 `c` 上。哪一处显式比较阻止了 `get(b)` 返回 `c` 的值？
* 包含与排除的上界分别在哪里施加？写一个边界测试，如果实现对两种边界都使用 `<`，该测试就会失败。

### 资源生命周期与性能

* 假设用户对整个 1 TB 的存储引擎创建迭代器，扫描大约持续一小时。这可能带来什么问题？我们会在课程的多个位置重新讨论这个问题。
* 一些 LSM 树存储引擎提供 multi-get（或称 vectored-get）接口。调用方提供一组键，接口为每个键返回一个值；例如 `multi_get(vec!["a", "b", "c", "d"]) -> a=1,b=2,c=3,d=4`。最简单的实现是对每个键各执行一次 `get`。你会如何实现 multi-get，哪些地方可以优化？提示：get 路径中的某些工作对整个批次只需做一次。你也可以考虑为 multi-get 设计改进的磁盘 I/O 接口。

用[第一周周末自检](./week1-overview.md#end-of-week-self-check)校验核心不变量。其余设计问题可能有多个站得住脚的答案；在比较取舍之前，先说明你的工作负载和假设。你也可以在 Discord 社区讨论。

## 附加任务

* **动态分发的代价。** 实现一个 `Box<dyn StorageIterator>` 版本的归并迭代器，并通过基准测试观察性能差异。
* **并行定位。** 创建归并迭代器时，每创建一个 `SsTableIterator` 都需要从对应的 SSTable 加载第一个相关块。考虑并行创建这些迭代器。

{{#include copyright.md}}
