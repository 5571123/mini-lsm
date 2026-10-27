<!--
  mini-lsm-book © 2022-2026 by Alex Chi Z is licensed under CC BY-NC-SA 4.0
-->

# 压缩实现

![本章概览](./lsm-tutorial/week2-01-full.svg)

学完本章，你将能够：

* 实现把若干文件合并并产出新文件的压缩逻辑。
* 安装压缩结果，同时不丢失并发落盘的 SST。
* 更新 LSM 读路径，以纳入范围互不重叠的有序段。
* 解释为什么迭代器优先级和目标层决定了一个墓碑（tombstone）能否被丢弃。

把测试用例复制到起始代码并运行：

```
cargo x copy-test --week 2 --day 1
cargo x scheck
```

<div class="warning">

开始本章之前，请先读[第二周概览](./week2-overview.md)，了解压缩与放大的背景。

</div>

## 开始之前

第一周结束时，每次落盘都会创建一个新的 L0 SST。L0 文件之间可以有重叠的键范围，并从新到旧存放。本章引入 L1，它是一个有序段，其中各 SST 的键范围互不重叠，并按各自的第一个键排序。

实现任务时，请记住这些不变量：

1. 当多个输入包含同一个键时，代表最新来源的迭代器必须胜出。
2. L0 可以重叠，但为 L1 产出的 SST 必须有序且互不重叠。
3. 只有当压缩包含该键所有可能更旧的版本时，墓碑（tombstone）才可以丢弃。在本章中，完整压缩的目标是底层，因此这个条件成立。
4. 合并与写出输出 SST 在 `state_lock` 之外进行。安装结果时持锁，并且只能移除任务点名的文件。任务创建之后落盘的 L0 SST 必须保留在新状态中。
5. 如果所有幸存条目都是墓碑（tombstone），压缩不产出 SST，而不是构建一个空 SST。

> **写代码前先预测：** 一个完整压缩任务选取了 L0 文件 `[5, 4]` 和 L1 文件 `[1, 2]`。在它写出输出 SST 期间，文件 6 被落盘到 L0 的最前面。安装结果之后，L0 里应保留哪些文件？如果文件 5 中 `k` 的最新值是一个墓碑（tombstone），而更旧的值在文件 1 中，两个条目是否应出现在输出里？

## 任务 1：压缩实现

本任务实现核心的压缩操作：把一组 SST 合并成一个有序段。你需要修改：

```
src/compact.rs
```

具体来说，实现 `force_full_compaction` 和 `compact` 两个函数。`force_full_compaction` 选择文件并把结果安装到 LSM 状态中。`compact` 执行合并并返回一组新的 SST。

用 `MergeIterator` 合并任务选取的每个 SST，然后通过 `SsTableBuilder` 写出幸存条目。输出达到目标 SST 大小时就切分。合并结束后，把输出安装为 L1 有序段，并从状态和文件系统中移除过期的输入。在这个阶段，SST 只出现在 L0 或 L1 中，因此 `LsmStorageState::levels` 只有一个元素。

压缩不应阻塞 L0 落盘。合并文件、写出输出时不要持有 `state_lock`。只在安装结果时获取它，并在结构更新和清单（manifest）记录完成之后立即释放。

你可以假设同一时刻只有一个压缩在运行。安装到 L1 的 SST 必须按第一个键排序，且键范围互不重叠。

<details>

<summary>剧透：压缩伪代码</summary>

```rust,no_run
fn force_full_compaction(&self) {
    let ssts_to_compact = {
        let state = self.state.read();
        state.l0_sstables + state.levels[0]
    };
    let new_ssts = self.compact(FullCompactionTask(ssts_to_compact))?;
    {
        let state_lock = self.state_lock.lock();
        let state = self.state.write();
        state.l0_sstables.remove(/* 正在被压缩的那些 */);
        state.levels[0] = new_ssts; // 新增的 SST 加入 L1
    };
    std::fs::remove(ssts_to_compact)?;
}
```

</details>

目前 `compact` 只需要处理 `ForceFullCompaction`，它的任务列出了输入的 SST。请保持来源优先级，让每个键的最新版本进入输出。

由于这个任务包含所有 SST，只需保留每个键的最新版本。如果该版本是墓碑（tombstone），就把它从输出中省略。后续章节只压缩树的一部分，因此不能总是丢弃墓碑。

继续之前，请考虑以下两种情况：

* 你的实现如何保留压缩进行期间落盘的 L0 SST？
* 在输入文件名被解除链接之后，使用较旧状态快照的读线程还能正常完成吗？在类 Unix 系统上，已打开的文件在最后一个句柄关闭之前仍然可访问。

## 任务 2：Concat 迭代器

本任务需要修改：

```
src/iterators/concat_iterator.rs
```

有序段不需要归并迭代器：它的 SST 有序且范围互不重叠，因此一个 concat 迭代器可以顺序访问它们。保存 SST 对象，并且只创建当前活跃的子迭代器。提前创建所有子迭代器会不必要地读取每个 SST 的第一个块。

## 任务 3：集成到读路径

本任务需要修改：

```
src/lsm_iterator.rs
src/lsm_storage.rs
src/compact.rs
```

现在更新两层的读路径，让 L1 使用 concat 迭代器。

修改 `LsmIterator` 的内层迭代器类型。先合并内存表和 L0 的迭代器，再用 `TwoMergeIterator` 把这条较新的流与 L1 的 concat 迭代器组合起来。

你也可以修改压缩实现，让它利用 concat 迭代器。

为 concat 迭代器实现 `num_active_iterators`。在本练习中，它应始终报告一个活跃迭代器。

要交互式地测试你的实现，

```shell
cargo run --bin mini-lsm-cli-ref -- --compaction none # 参考实现
cargo run --bin mini-lsm-cli -- --compaction none # 你的实现
```

然后，

```
fill 1000 3000
flush
fill 1000 3000
flush
full_compaction
fill 1000 3000
flush
full_compaction
get 2333
scan 2000 2333
```

## 本章检查点

你的引擎现在应能把任务选取的 L0 和 L1 输入压缩为零个或多个有序的 L1 SST，保留任务快照之后创建的 L0 文件，并通过重叠与不重叠两类来源同时服务 `get` 和 `scan`。

除了通过测试，还要显式验证三种情况：

1. 让两个输入 SST 包含同一个键，确认颠倒它们的合并优先级会改变结果。
2. 压缩一个只含墓碑（tombstone）的输入，确认不会创建空 SST。
3. 把 concat 迭代器定位到第二个或更靠后的 SST，确认它只打开活跃的子迭代器。

## 检验你的理解

请用具体的 LSM 状态或执行过程回答正确性问题。对于放大相关的问题，说明分子和分母各计入什么。

### 正确性与并发

* 构造最小的输入，使颠倒 L0 迭代器优先级时保留一个过期值。再构造一个使已删除的值重新出现的输入。
* 在本章的完整压缩中，为什么丢弃墓碑（tombstone）是安全的？给出一个反例，说明在压缩到非底层时同一规则为何不安全。
* 你的实现如何在压缩写出输出期间保留新落盘的 L0 SST？
* 当压缩不产出任何 SST 时，`apply_compaction_result` 应该做什么？
* 如果你的实现在安装新状态后立即删除原 SST 文件，使用较旧状态快照的读线程还能完成吗？答案如何取决于文件系统语义？
* 在可以安全使用 `SstConcatIterator` 之前，必须满足哪些顺序和不重叠性质？

### 性能与设计

* 读放大、写放大、空间放大的定义分别是什么？（概览章节已经讲过）
* 有哪些方法可以精确计算读放大、写放大、空间放大，又有哪些方法可以估算它们？
* 即使请求删除某个键，该键仍会占用一些存储空间，这个说法正确吗？
* 由于压缩消耗读写带宽，引擎是否应在前台流量较重时推迟或暂停它？这会带来什么新问题？请阅读 [SILK: Preventing Latency Spikes in Log-Structured Merge Key-Value Stores](https://www.usenix.org/conference/atc19/presentation/balmau)。
* 让压缩使用或填充块缓存是好主意吗？还是在压缩时完全绕过块缓存更好？
* 在系统中放一个 `struct ConcatIterator<I: StorageIterator>` 有意义吗？
* 一些研究者和工程师提出把压缩卸载到远程服务器或无服务器 lambda 函数上。这样做有什么好处，可能遇到哪些挑战和性能影响？（想一想压缩完成时的那一刻，以及下一次读请求时块缓存会发生什么……）

用[第二周周末自检](./week2-overview.md#end-of-week-self-check)校验核心不变量。其余设计问题可能有多个站得住脚的答案；在比较取舍之前，先说明你的工作负载和假设。你也可以在 Discord 社区讨论。

{{#include copyright.md}}
