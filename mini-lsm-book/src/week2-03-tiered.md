<!--
  mini-lsm-book © 2022-2026 by Alex Chi Z is licensed under CC BY-NC-SA 4.0
-->

# 分层（tiered）压缩策略

![本章概览](./lsm-tutorial/week2-00-tiered.svg)

学完本章，你将能够：

* 实现分层（tiered）压缩策略，并在压缩模拟器上模拟它。
* 把分层（tiered）压缩纳入引擎的落盘、压缩和读路径。
* 解释 universal 压缩的三个触发条件如何取舍写放大、读放大和空间放大。
* 保持从新到旧的层（tier）顺序，并判断一个任务是否包含最底部的层（tier）。

本章的分层（tiered）策略就是 RocksDB 的 universal 压缩。本章混用这两个术语。

把测试用例复制到起始代码并运行：

```
cargo x copy-test --week 2 --day 3
cargo x scheck
```

<div class="warning">

开始本章之前，请先读[第二周概览](./week2-overview.md)，了解压缩与放大的背景。

</div>

## 开始之前

与分层（leveled）压缩不同，分层（tiered）压缩把每次落盘都视为一个新的有序段。`levels` 向量改用来从新到旧存放层（tier）；数字 ID 只是一个层的名字，并不描述固定的深度。

请记住这些不变量：

1. `levels[0]` 是最新的层（tier），每个层内部有序且范围互不重叠。
2. 分层（tiered）任务选取一段连续的层（tier）前缀。归并迭代器必须使用同样的从新到旧顺序，使较新的版本胜出。
3. 控制器按顺序考虑触发条件：最小层数、空间放大、大小比例，最后是减少有序段数量。
4. 只有当被选取的前缀包含最旧的层（tier）时，`bottom_tier_included` 才为真。如果留下了更旧的层，墓碑（tombstone）必须保留。
5. 落盘会在 `levels` 的前端创建一个只含一个 SST 的层（tier）；它不会添加 L0 文件。

> **写代码前先预测：** 假设从新到旧的各层（tier）大小为 `[1, 1, 3, 20]`，且减少有序段的触发条件被 `max_merge_width = 2` 限制。会选取哪些层？该任务是否包含最底部的层，它能否丢弃一个墓碑（tombstone）——而该墓碑更旧的值可能位于那个 20 个文件的层中？

## 任务 1：Universal 压缩

你将实现 RocksDB universal 压缩的一个简化形式。与简单分层压缩一样，控制器使用文件数而不是字节大小。一个任务总是完整包含每个被选取的层（tier）。

### 任务 1.0：前置条件

本任务需要修改：

```
src/compact/tiered.rs
```

在 universal 压缩中，LSM 状态不使用 L0。每次内存表落盘都创建一个只含一个 SST 的有序段，也就是一个层（tier）。`levels` 向量保存所有层，**最新的层位于最低的下标**。每个元素是一个元组，包含一个层 ID 和该层中的 SST ID。把每个新落盘的 SST 放进位于向量前端的层中。压缩模拟器把第一个输出 SST 的 ID 用作层 ID；你的实现也应如此。

只有当层（tier）数（有序段数）达到 `num_tiers` 时，universal 压缩才会触发任务。否则不触发任何压缩。

### 任务 1.1：由空间放大比例触发

universal 压缩的第一个触发条件是空间放大比例。正如概览章节所讨论的，空间放大可以用 `engine_size / last_level_size` 估算。在我们的实现中，空间放大比例用 `除最后一层外所有层的大小 / 最后一层的大小` 计算，这样比值可以落在 `[0, +inf)` 而不是 `[1, +inf)`。这也与 RocksDB 的实现一致。

这个估算建模的是一个逻辑大小固定的数据集——例如 100 GB——它不断收到更新。最底部的层（tier）近似这个逻辑数据集，而上层（tier）包含尚未到达底部的较新变更。实现把多余空间表示为 `upper_tier_size / bottom_tier_size`，取值范围从零向上。

当 `upper_tier_size / bottom_tier_size` 至少为 `max_size_amplification_percent / 100` 时，触发完整压缩。例如：

```
Tier 3: 1
Tier 2: 1 ; 除最后一层外所有层的大小 = 2
Tier 1: 1 ; 最后一层大小 = 1，2/1=2
```

假设 `max_size_amplification_percent` = 200，我们现在应触发一次完整压缩。

实现这个触发条件之后，你可以运行压缩模拟器。你会看到：

```shell
cargo run --bin compaction-simulator tiered --iterations 10
```

```
=== Iteration 2 ===
--- After Flush ---
L3 (1): [3]
L2 (1): [2]
L1 (1): [1]
--- Compaction Task ---
compaction triggered by space amplification ratio: 200
L3 [3] L2 [2] L1 [1] -> [4, 5, 6]
--- After Compaction ---
L4 (3): [3, 2, 1]
```

如果只实现这一个触发条件，模拟结束时会出现类似这样的状态：

```bash
cargo run --bin compaction-simulator tiered
```

```
=== Iteration 7 ===
--- After Flush ---
L8 (1): [8]
L7 (1): [7]
L6 (1): [6]
L5 (1): [5]
L4 (1): [4]
L3 (1): [3]
L2 (1): [2]
L1 (1): [1]
--- Compaction Task ---
--- Compaction Task ---
compaction triggered by space amplification ratio: 700
L8 [8] L7 [7] L6 [6] L5 [5] L4 [4] L3 [3] L2 [2] L1 [1] -> [9, 10, 11, 12, 13, 14, 15, 16]
--- After Compaction ---
L9 (8): [8, 7, 6, 5, 4, 3, 2, 1]
--- Compaction Task ---
1 compaction triggered in this iteration
--- Statistics ---
Write Amplification: 16/8=2.000x
Maximum Space Usage: 16/8=2.000x
Read Amplification: 1x

=== Iteration 49 ===
--- After Flush ---
L82 (1): [82]
L81 (1): [81]
L80 (1): [80]
L79 (1): [79]
L78 (1): [78]
L77 (1): [77]
L76 (1): [76]
L75 (1): [75]
L74 (1): [74]
L73 (1): [73]
L72 (1): [72]
L71 (1): [71]
L70 (1): [70]
L69 (1): [69]
L68 (1): [68]
L67 (1): [67]
L66 (1): [66]
L65 (1): [65]
L64 (1): [64]
L63 (1): [63]
L62 (1): [62]
L61 (1): [61]
L60 (1): [60]
L59 (1): [59]
L58 (1): [58]
L57 (1): [57]
L33 (24): [32, 31, 30, 29, 28, 27, 26, 25, 24, 23, 22, 21, 20, 19, 18, 17, 9, 10, 11, 12, 13, 14, 15, 16]
--- Compaction Task ---
--- Compaction Task ---
no compaction triggered
--- Statistics ---
Write Amplification: 82/50=1.640x
Maximum Space Usage: 50/50=1.000x
Read Amplification: 27x
```

模拟器把 `num_tiers` 设为 8，但状态仍可能远远超过八个层（tier），因为该阈值只是允许调度；空间触发条件并不保证一定会选中一个任务。这会造成高读放大。

当前的触发条件只控制空间放大。接下来的两个触发条件限制读放大。

### 任务 1.2：由大小比例触发

大小比例触发条件让各层（tier）之间保持几何增长。从最新的层开始，把下一个层的大小与所有更新层的总大小比较。在第一个比值大于 `(100 + size_ratio) / 100` 的位置，压缩较新的前缀，但排除满足该比值的那个层。只有当该前缀至少包含 `min_merge_width` 个层时才调度任务。

例如，取 `size_ratio = 1` 和 `min_merge_width = 2`，当比值超过 101% 时压缩：

```
Tier 3: 1
Tier 2: 1 ; 1 / 1 = 1
Tier 1: 1 ; 1 / (1 + 1) = 0.5，不触发压缩
```

例子 2：

```
Tier 3: 1
Tier 2: 1 ; 1 / 1 = 1
Tier 1: 3 ; 3 / (1 + 1) = 1.5，压缩层 2+3
```

```
Tier 4: 2
Tier 1: 3
```

例子 3：

```
Tier 3: 1
Tier 2: 2 ; 2 / 1 = 2，但只压缩一个层没有意义；另注意 min_merge_width=2
Tier 1: 4 ; 4 / 3 = 1.33，压缩层 2+3
```

```
Tier 4: 3
Tier 1: 4
```

加上这个触发条件后，你会在压缩模拟器中观察到：

```bash
cargo run --bin compaction-simulator tiered
```

```
=== Iteration 49 ===
--- After Flush ---
L119 (1): [119]
L118 (1): [118]
L114 (4): [113, 112, 111, 110]
L105 (5): [104, 103, 102, 101, 100]
L94 (6): [93, 92, 91, 90, 89, 88]
L81 (7): [80, 79, 78, 77, 76, 75, 74]
L48 (26): [47, 46, 45, 44, 43, 37, 38, 39, 40, 41, 42, 24, 25, 26, 27, 28, 29, 30, 9, 10, 11, 12, 13, 14, 15, 16]
--- Compaction Task ---
--- Compaction Task ---
no compaction triggered
--- Statistics ---
Write Amplification: 119/50=2.380x
Maximum Space Usage: 52/50=1.040x
Read Amplification: 7x
```

```bash
cargo run --bin compaction-simulator tiered --iterations 200 --size-only
```

```
=== Iteration 199 ===
--- After Flush ---
Levels: 0 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 2 3 4 5 6 10 15 21 28 78
no compaction triggered
--- Statistics ---
Write Amplification: 537/200=2.685x
Maximum Space Usage: 200/200=1.000x
Read Amplification: 38x
```

这个触发条件产出的单 SST 层（tier）更少，并且倾向于让各层从小到大排列。状态仍可能超过 `num_tiers`，因此还需要最后一个触发条件。

### 任务 1.3：减少有序段数量

如果前面的触发条件都没有产出任务，就把前 `max_merge_width` 个层（tier）合并成一个层，以减少有序段的数量。如果没有设置 `max_merge_width`，则选取所有层。请记住，当还有更旧的层留下时，被限制的前缀不包含最底部的层。

启用这个压缩触发条件后，你会看到：

```bash
cargo run --bin compaction-simulator-ref tiered --iterations 200 --size-only
```

```
=== Iteration 199 ===
--- After Flush ---
Levels: 0 1 1 4 5 21 28 140
no compaction triggered
--- Statistics ---
Write Amplification: 742/200=3.710x
Maximum Space Usage: 280/200=1.400x
Read Amplification: 7x
```

你也可以试试把层数上限调大后的分层（tiered）压缩：

```bash
cargo run --bin compaction-simulator tiered --iterations 200 --size-only --num-tiers 16
```

```
=== Iteration 199 ===
--- After Flush ---
Levels: 0 1 1 1 1 1 1 1 1 1 1 15 175
no compaction triggered
--- Statistics ---
Write Amplification: 607/200=3.035x
Maximum Space Usage: 350/200=1.750x
Read Amplification: 12x
```

**注意：这一部分我们不提供细粒度的单元测试。你可以运行压缩模拟器，并与参考实现的输出比较，判断你的实现是否正确。**

## 任务 2：集成到读路径

本任务需要修改：

```
src/compact.rs
src/lsm_storage.rs
```

分层（tiered）压缩不使用 L0，因此把每个内存表直接落盘为一个新的层（tier）。用 `self.compaction_controller.flush_to_l0()` 在分层（leveled）和分层（tiered）两条落盘路径之间选择。用第一个输出 SST 的 ID 为压缩产出的层命名，并按从新到旧的层顺序构造压缩用的归并迭代器。

## 本章检查点

你的引擎现在应能直接落盘到按最新优先排列的层（tier），调度第一个符合条件的 universal 压缩触发条件，并在任务留下更旧的层时保留墓碑（tombstone）。当所有幸存条目都是墓碑时，包含最底部层的任务可以不产出任何 SST。

对模拟器的若干次迭代，为每个任务标注：选中它的触发条件、它消耗的层，以及它是否到达底部。然后用一个重复键验证跨层的合并优先级。最后测试受限的 `max_merge_width`，确认任务不会声称包含一个未被选取的底层。

## 延伸阅读

[Universal Compaction - RocksDB Wiki](https://github.com/facebook/rocksdb/wiki/Universal-Compaction)

## 检验你的理解

### 正确性与调度

* 为什么每个任务都必须从状态的最新一端选取相邻的层？构造一个反例：任务合并两个不相邻的层并把输出放错位置。
* 当 `max_merge_width` 把任务限制为只取最新的若干层时，为什么 `bottom_tier_included` 必须为假？
* 如果底层压缩因为所有幸存条目都是墓碑（tombstone）而没有输出，新的 LSM 状态应包含什么？
* 构造一个层大小序列，使空间放大触发条件胜出；再构造一个使大小比例触发条件胜出。
* 如果任务运行期间落盘了一个新的层（tier），压缩输出应插在这个层的哪个位置？

### 放大与设计

* 针对一个具体的层大小序列估算写放大。先忽略最后的*减少有序段数量*触发条件，用写出的物理 SST 字节数除以落盘字节数。
* 在一个有 `N` 个层（tier）、且没有缓存和布隆过滤器收益的状态下，估算最坏情况的点查放大。哪个策略参数限定了这个值？
* 与分层（leveled）压缩相比，universal 压缩有哪些优点和缺点？
* 一个 universal 压缩任务需要多少临时空闲存储？请针对一组具体的输入层大小作答，并区分任务的峰值空间与稳态空间放大。
* 如果压缩速度跟不上分层（tiered）压缩的 SST 落盘速度，会发生什么？
* 系统在并行调度多个压缩任务之前必须考虑什么？
* SSD 自己也会写日志（基本上它就是一种日志结构存储）。如果 SSD 的写放大是 2x，整个系统的端到端写放大是多少？相关阅读：[ZNS: Avoiding the Block Interface Tax for Flash-based SSDs](https://www.usenix.org/conference/atc21/presentation/bjorling)。
* 考虑用户为分层（tiered）压缩保留大量有序段（例如 300 个）的情况。为了让读路径更快，是否可以维护某种数据结构，把在每一层中查找某键范围需读取的 SST 的时间复杂度降到 `O(log n)`？注意，通常你需要在每个有序段中二分查找，才能找到需要读取的键范围。（看看 Neon 的 [layer map](https://neon.tech/blog/persistent-structures-in-neons-wal-indexing) 实现！）

用[第二周周末自检](./week2-overview.md#end-of-week-self-check)校验核心不变量。其余设计问题可能有多个站得住脚的答案；在比较取舍之前，先说明你的工作负载和假设。你也可以在 Discord 社区讨论。

{{#include copyright.md}}
