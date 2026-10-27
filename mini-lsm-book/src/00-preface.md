<!--
  mini-lsm-book © 2022-2026 by Alex Chi Z is licensed under CC BY-NC-SA 4.0
-->

# 学习数据库内部原理

![Mini-LSM：用 Rust 构建 LSM 存储引擎，学习数据库内部原理](./mini-lsm-banner.svg)

Mini-LSM 是一门面向系统与后端工程师的实践课程。你将用 Rust 构建一个 LSM-tree 存储引擎，覆盖从内存写路径、磁盘格式，到压缩（compaction）、崩溃恢复、MVCC 与事务的完整链路。

第一周产出一个可用的存储引擎。第二周和第三周在此基础上加入贴近生产环境的持久化、后台维护与并发控制。如果你使用编码智能体，还有一条独立的引导式路线：由智能体承担大部分机械性的实现工作，而你仍然掌握数据结构表示、顺序、失败处理与测试这些关键决策。

## 选择你的路线

| 路线 | 形式 | 从这里开始 |
| --- | --- | --- |
| **引导式课程** | 三周，每周七个实现章节 | [阅读课程概览](./00-overview.md) |
| **编码智能体路线** | 三个引导日，包含决策节点、小段代码与对抗性测试；第 1–3 天已可用 | [准备你的智能体并开始](./agent-fast-forward-overview.md) |

两条路线构建的是同一个微型存储系统。区别在于大部分代码由谁敲，而不是由谁负责理解。

## 这门课适合你吗？

如果你用过 PostgreSQL、MySQL、Redis、RocksDB 或其他数据系统，并且想了解它的 API 之下发生了什么，Mini-LSM 就是为你设计的。如果你希望读懂生产级存储引擎的代码，但首先需要一个完整到能装进脑子里的系统，这门课也是一个实用的起点。

你将学到：

* 存储布局、内存中的有序状态、块（block）与 SST；
* 跨多个物理数据源的读路径与写路径；
* 压缩（compaction）及其在读、写、空间三方面的权衡；
* 通过清单（manifest）、预写日志（WAL）与恢复实现的持久化；以及
* 快照、MVCC、垃圾回收与并发控制。

Mini-LSM 聚焦于数据库的**存储引擎**。它不包含 SQL 解析器、查询优化器、复制或分布式共识。这些都是有价值的后续内容，但把它们排除在课程之外，才可能端到端地理解存储层。

## 什么是 LSM 树，为什么要用它？

日志结构合并树（log-structured merge tree）是维护键值对的数据结构。它们被广泛用作分布式数据库系统的底层
存储引擎，例如 [TiDB](https://www.pingcap.com) 和
[CockroachDB](https://www.cockroachlabs.com)。[RocksDB](http://rocksdb.org) 基于
[LevelDB](https://github.com/google/leveldb)，是一个 LSM-tree 存储引擎，提供丰富的键值接口，
被许多生产系统使用。

一般来说，LSM 树是一种对追加友好的数据结构。把它与其他键值数据结构（例如红黑树和 B 树）对比，最容易理解。
在那些结构中，更新是原地发生的：当你更新某个键对应的值时，引擎会在该值原本所在的内存或磁盘位置覆盖它。而在
LSM 树中，写入——包括插入、更新和删除——是延迟施加到持久化存储上的。引擎把这些操作攒成
sorted-string table（SST）文件并写入磁盘。SST 文件一旦写入就不可变。一个称为压缩（compaction）的后台过程会合并这些文件，并施加其中的更新与删除。

这种架构设计带来几个有用的工程性质。

1. 持久化存储上的数据不可变，这使并发控制更简单。压缩可以卸载到远程服务器，数据也可以直接存储并由 S3 这类云原生存储系统提供。
2. 更换压缩算法可以让存储引擎在读放大、写放大和空间放大之间取得平衡。通过调节压缩参数，我们可以针对不同负载优化 LSM 树。

## 前置知识

* 你应当了解 Rust 编程语言的基础。读完 [The Rust Programming Language](https://doc.rust-lang.org/book/) 就够了。课程会在遇到较冷门的 Rust 模式时介绍它们，但你应当能从容阅读编译器报错并查阅 crate 文档。
* 你应当理解 Bitcask 这类小型持久化键值存储：把更新追加到日志、维护指向最新记录的内存索引、用标记表示删除、通过合并文件回收过期记录，以及重启后重建状态。如果这些概念对你来说陌生，可以考虑通过 [PingCAP Talent Plan](https://github.com/pingcap/talent-plan/tree/master/courses/rust/projects/project-2) 实现 Bitcask。
* 你**不**需要预先了解 LSM 树、压缩策略、MVCC 或事务隔离级别。[课程概览](./00-overview.md)会把 Bitcask 模型过渡到 Mini-LSM，每周也会各自介绍其存储概念。外部的 LevelDB 或 RocksDB 概览只是可选拓展，不是必需的准备。

## 你应该期待从这门课得到什么？

完成这门课后，你应当深入理解基于 LSM 的存储系统如何工作，并具备设计一个存储系统的实践经验。你会学到其中的权衡，以及如何选择满足特定负载需求的设计。基于作者在多个 LSM 系统上的经验，课程覆盖了 RocksDB 这类现代存储系统中的关键实现细节与设计取舍。你学到的内容可以用于工业界与学术界。

### 结构

课程由几个部分组成，也就是周。每周有七个章节，每章可以在两到三个小时内完成。每周的前六章引导你构建一个可运行的系统。最后一章是*加餐时间*章节，你会在前六天成果的基础上实现几项难度适中的改进。每章都包含必做任务、*检验你的理解*问题以及加分任务。

### 如何使用这些学习检查

本书使用四类检查，用途各不相同：

1. **编码前先预测**要求你在实现细节分散注意力之前推演一个小状态。如果之后的代码与预测不符，先判断是你的预测还是你的实现漏掉了哪个不变量。
2. **章节检查点**把测试与自带测试套件可能未覆盖的情形结合起来。记录支撑你结论的状态、执行过程或字节布局；仅凭测试通过不足以作为证据。
3. **检验你的理解**先提出在本章模型内有确定答案的问题，再扩展到性能与生产设计类问题。对于开放式问题，先说明负载与系统假设，再为某个权衡辩护。
4. **周末自检**提供小型综合场景，并附可折叠的答案标准。它们用于在回答章节问题之后做校准，而不是实现步骤清单。

需要外部 API 细节或论文的问题应当链接到相关的一手资料。如果某个问题看起来仍然描述不清，请指出缺失的假设，而不是猜一个通用答案。

### 测试

我们提供了一套完整的测试套件和若干命令行工具，帮助你验证自己的实现。测试套件并不穷尽所有情况，因此通过全部测试并不保证你的实现完全正确。在实现系统后续部分时，你可能需要修复早期的 bug。请仔细思考你的实现，尤其是涉及多线程操作与潜在竞态条件时。

### 参考解

Mini-LSM 主仓库包含一个参考解，实现了课程要求的全部功能。我们还维护一个参考解检查点仓库，其中每个提交对应一个章节。

让检查点仓库与 Mini-LSM 课程保持同步很困难，因为每个 bug 修复或新功能都必须应用到所有相关提交上。因此，该仓库可能没有使用最新的起始代码，也可能不包含课程的最新特性。

**TL;DR：我们不保证参考解检查点仓库包含正确的解、能通过全部测试，或拥有准确的文档注释。** 完整参考实现见[主仓库中的 `mini-lsm` crate](https://github.com/skyzh/mini-lsm/tree/main/mini-lsm)。

如果你卡住了，或者需要确定该在哪里实现功能，可以参考检查点仓库。对比相邻提交，就能看到每个章节改了什么。由于课程中你会多次修改某些函数，这些 diff 能清楚说明每个章节期望你实现什么。

你可以在这里访问参考解检查点仓库：[https://github.com/skyzh/mini-lsm-solution-checkpoint](https://github.com/skyzh/mini-lsm-solution-checkpoint)。

### 反馈

非常感谢你的反馈。2024 年，我们根据学生的反馈把整门课程从零重写了一遍。请在 [Discord 社区](https://skyzh.dev/join/discord) 分享你的学习体验，帮助我们继续改进课程。

下面是重写背后的完整故事。原课程只给出大致指引：学生从空目录开始，按我们的规格实现自己的设计，再用一套最小测试检查行为。这种做法过于开放，给学习造成了明显障碍。缺少对完整系统的概览，加上说明有时含糊，学生很难理解某个设计决策为何做出，也很难知道自己要完成什么。有些小节也过于密集，难以舒适地塞进一个章节。

因此我们重新设计了课程，让学习曲线更平缓、目标更清晰。原来一周的课程拆成两周：第一周讲存储格式，第二周深入压缩。第三部分讲 MVCC。希望你觉得这门课在学习与职业中都既有意思又有用。我们感谢所有在 [Feedback after coding day 1](https://github.com/skyzh/mini-lsm/issues/11) 和 [Hello, when is the next update plan for the course?](https://github.com/skyzh/mini-lsm/issues/7) 中留言的人——你的反馈极大地帮助我们改进了课程。

### 许可

本课程的源代码采用 Apache 2.0 许可，本书采用 CC BY-NC-SA 4.0 许可。

### 这门课会永远免费吗？

会！目前公开可用的内容将保持免费，并持续获得更新与 bug 修复。我们也可能提供付费的代码审查与答疑服务。截至 2024 年，我们尚无完成可下载内容部分（*你的余生*章节）的计划，也尚未决定这部分是否公开。

## 社区

你可以加入 skyzh 的 Discord 服务器，与 mini-lsm 社区一起学习。

[![加入 skyzh 的 Discord 服务器](discord-badge.svg)](https://skyzh.dev/join/discord)

## 开始学习

接下来，请阅读 [Mini-LSM 课程概览](./00-overview.md)，了解 LSM 结构。

## 关于作者

在 2024 年初写作本书时，Chi 拥有卡内基梅隆大学计算机科学硕士学位和上海交通大学学士学位。他参与过多个数据库系统，包括 [TiKV][db1]、[AgateDB][db2]、[TerarkDB][db3]、[RisingWave][db4] 和 [Neon][db5]。从 2022 年起，他连续三个学期担任 [CMU 的数据库系统课程](https://15445.courses.cs.cmu.edu/) 助教，参与 BusTub 教学系统的工作。在那里，他为课程添加了新的功能与挑战，包括重新设计的[查询执行](https://15445.courses.cs.cmu.edu/fall2022/project3/)项目和难度较高的[多版本并发控制](https://15445.courses.cs.cmu.edu/fall2023/project4/)项目。他还维护教学数据库系统 [RisingLight](https://github.com/risinglightdb/risinglight)。Chi 对探索 Rust 在数据库领域的作用很感兴趣。如果你也有同样的兴趣，可以看看他之前的两门课程：构建向量化表达式框架的 [type-exercise-in-rust](https://github.com/skyzh/type-exercise-in-rust)，以及构建向量数据库的 [write-you-a-vector-db](https://github.com/skyzh/write-you-a-vector-db)。

[db1]: https://github.com/tikv/tikv
[db2]: https://github.com/tikv/agatedb
[db3]: https://github.com/bytedance/terarkdb
[db4]: https://github.com/risingwavelabs/risingwave
[db5]: https://github.com/neondatabase/neon

{{#include copyright.md}}
