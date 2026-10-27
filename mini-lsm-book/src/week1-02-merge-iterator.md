<!--
  mini-lsm-book © 2022-2026 by Alex Chi Z is licensed under CC BY-NC-SA 4.0
-->

# 归并迭代器

![本章概览](./lsm-tutorial/week1-02-overview.svg)

学完本章，你将能够：

* 实现内存表迭代器。
* 实现归并迭代器。
* 实现 LSM `scan` 读路径中处理内存表的部分。
* 解释游标有效性、来源优先级和墓碑过滤如何共同产生一个有序的逻辑视图。

把测试用例复制到起始代码并运行：

```
cargo x copy-test --week 1 --day 2
cargo x scheck
```

## 开始之前

引擎现在可以在多个内存表上解析一次点查，但还不能返回一段键范围。扫描必须在不把各个来源的全部内容物化到内存的前提下，合并多个各自有序的来源。

迭代器栈把这一职责分层：

- `MemTableIterator` 暴露单个有序内存表。
- `MergeIterator` 合并同类型的来源，去掉重复版本，并让下标更小——也就是更新——的来源优先。
- `LsmIterator` 从用户可见的流中移除墓碑。
- `FusedIterator` 让失效状态和错误状态对调用方是安全的。

核心不变量是：**输出有序，每个用户键最多出现一次，取值为包含该键的最新输入中的值。** 在 `LsmIterator` 隐藏墓碑之前，墓碑同样参与优先级判定。

> **写代码前先预测：** 把 `iter1 = [b->delete, c->4]` 与 `iter2 = [a->1, b->2, c->3]` 合并，其中 `iter1` 更新。先写出包含墓碑的内部归并流，再写出用户可见的流。如果在裁决重复版本之前就移除墓碑，会破坏什么？

## 任务 1：内存表迭代器

本章实现 LSM 的 `scan` 接口，它用迭代器返回一段有序的键值对。上一章你实现了 `get` 以及创建只读内存表的逻辑，因此 LSM 状态现在可以包含多个内存表。你先为单个内存表创建迭代器，再合并所有内存表的迭代器，最后施加请求的键范围。

本任务需要修改：

```
src/mem_table.rs
```

所有 LSM 迭代器都实现 `StorageIterator` trait。它的四个核心方法是 `key`、`value`、`next` 和 `is_valid`。如果你熟悉 Rust 标准库的 `Iterator` trait，会注意到 `StorageIterator` 的工作方式不同。它使用基于游标的 API，这种模式在数据库系统中很常见，灵感来自 RocksDB 的迭代器。可以参考 [`iterator_base.h`](https://github.com/facebook/rocksdb/blob/main/include/rocksdb/iterator_base.h) 和 [`iterator.h`](https://github.com/facebook/rocksdb/blob/main/include/rocksdb/iterator.h)。

迭代器创建时，游标指向内存表、块或 SSTable 中满足下界的第一个条目。`value` 方法返回 `&[u8]`，而 `key` 返回迭代器关联的借用键类型——例如 `KeySlice`。这些借用返回值避免了复制键和值的数据。

从调用方的角度看，典型用法是：

```rust
let mut iter: impl StorageIterator = ...;
while iter.is_valid() {
    let key = iter.key();
    let value = iter.value();
    // 处理键和值
    iter.next()?; // 前进到下一个元素，并处理可能出现的错误
}
```

`StorageIterator` 的核心方法各有分工：

* `next()`：尝试把游标移到下一个元素。它返回 `Result`，以便报告 I/O 失败这类错误。调用成功并不保证新位置有效；游标可能已经到达末尾。
* `is_valid()`：报告当前游标是否指向有效的数据元素。它*不*推进迭代器。

每次调用 `next()`（包括成功的调用）之后，你的实现都必须更新内部状态，使 `is_valid()` 能准确报告新的游标位置是否还有元素。

总之，`next` 推进游标，`is_valid` 报告该游标是否仍指向某个元素。你可以假设调用方只在 `is_valid` 返回 `true` 时调用 `next`。稍后你会实现 `FusedIterator` 包装器，统一迭代器失效或返回错误之后的行为。

现在回到内存表迭代器。你可能已经注意到它的公开类型没有生命周期参数。如果你创建一个 `Vec<u64>` 并调用 `vec.iter()`，迭代器类型会带有生命周期，例如 `VecIterator<'a>`，其中 `'a` 与向量绑定。`SkipMap::iter` 也是如此。但 Mini-LSM 避免在存储迭代器上暴露这种生命周期，因为那会让整个系统复杂化。

由于迭代器没有生命周期参数，它必须保证*底层跳表在迭代器使用期间一直存活*。做法是在迭代器内部保存 `Arc<SkipMap>`。第一版尝试可能写成这样：

```rust,no_run
pub struct MemtableIterator {
    map: Arc<SkipMap<Bytes, Bytes>>,
    iter: SkipMapRangeIter<'???>,
}
```

问题在于，我们需要表达 `iter` 借用了同一结构中的另一个字段 `map`。这个关系该怎么表示？

这是本课程第一个比较棘手的 Rust 概念：自引用结构体。如果 Rust 允许我们写出下面这样的代码，问题就解决了：

```rust,no_run
pub struct MemtableIterator { // <- 带生命周期 'this
    map: Arc<SkipMap<Bytes, Bytes>>,
    iter: SkipMapRangeIter<'this>,
}
```

`ouroboros` 这类第三方 crate 提供了安全接口来定义这种自引用结构体。你也可以用 unsafe Rust 实现；事实上 `ouroboros` 内部就使用了 unsafe Rust。

我们已经用 [`ouroboros`](https://docs.rs/ouroboros/latest/ouroboros/attr.self_referencing.html) 为你定义好 `MemTableIterator` 的自引用字段。请基于给定的结构实现它的迭代逻辑和 `MemTable::scan` API。

## 任务 2：归并迭代器

本任务需要修改：

```
src/iterators/merge_iterator.rs
```

由于 LSM 状态可能包含多个内存表，一次扫描会创建多个内存表迭代器。请合并它们的结果，并且每个键只返回最新的版本。

`MergeIterator` 内部维护一个二叉堆。二叉堆是合并 `n` 个有序迭代器的自然选择，因为它能高效找出当前键最小的那个迭代器。堆把当前键最小的迭代器排在前面。当多个迭代器的当前键相同时，更新者排在前面。请小心处理错误和已耗尽的迭代器，并确保归并只输出每个键值对的最新版本。

例如，假设有如下数据：

```
iter1: b->del, c->4, d->5
iter2: a->1, b->2, c->3
iter3: e->4
```

归并迭代器应输出：

```
a->1, b->del, c->4, d->5, e->4
```

归并迭代器的构造函数接受一个迭代器向量。下标更小——更靠近向量前端——的迭代器包含更新的数据。

使用 Rust 的二叉堆时，`peek_mut` 可能很有用：

```rust,no_run
let Some(mut inner) = heap.peek_mut() {
    *inner += 1; // 修改堆顶元素。
}
// PeekMut 被丢弃时，二叉堆会自动重新排序。

let Some(mut inner) = heap.peek_mut() {
    PeekMut::pop(inner) // 从堆中移除堆顶元素。
}
```

一个常见陷阱与错误处理有关。例如：

```rust,no_run
let Some(mut inner_iter) = self.iters.peek_mut() {
    inner_iter.next()?; // 有问题。
}
```

如果 `next` 返回错误——例如磁盘、网络或校验和失败——这个迭代器就不能再留在堆里。但作用域退出时，`PeekMut::drop` 会尝试恢复堆序，可能访问到这个无效迭代器。请显式处理错误并移除该迭代器，不要在 `PeekMut` 守卫存活期间使用 `?`。

在可行之处我们避免动态分发，因此系统不使用 `Box<dyn StorageIterator>`，而是使用泛型和静态分发。`StorageIterator` 还为其借用键类型使用了泛型关联类型（GAT），使不同迭代器可以返回 `KeySlice` 或 `&[u8]` 等类型。第三周会给 `KeySlice` 加上时间戳；现在就引入键的抽象能让这一过渡更平滑。

从这一节开始，LSM 的键使用 `Key<T>`，以便类型系统区分键和值。请使用提供的 `Key<T>` 方法，不要直接访问被包装的值。第三周会给这个类型加上时间戳，这层抽象能让过渡更平滑。目前 `KeySlice` 包装 `&[u8]`，`KeyVec` 包装 `Vec<u8>`，`KeyBytes` 包装 `Bytes`。

## 任务 3：LSM 迭代器 + Fused 迭代器

本任务需要修改：

```
src/lsm_iterator.rs
```

`LsmIterator` 表示存储引擎的内部迭代器。随着你添加更多迭代器类型，它会经历多次修改。目前引擎只包含内存表，因此它的内部类型应为：

```rust,no_run
type LsmIteratorInner = MergeIterator<MemTableIterator>;
```

实现 `LsmIterator`：把操作委托给内部迭代器，并跳过删除墓碑。

本任务不会直接测试 `LsmIterator`；任务 4 包含一个集成测试。

接下来，为迭代器的误用加上防护。迭代器失效时调用方不得调用 `key` 或 `value`，`next` 返回错误后也不得继续使用它。`FusedIterator` 包装另一个迭代器，统一这些行为。请按起始代码中记录的约定实现它。

## 任务 4：读路径——Scan

本任务需要修改：

```
src/lsm_storage.rs
```

有了这些迭代器，你就可以实现 LSM 引擎的 `scan` 接口。用内存表迭代器构造 `LsmIterator`，并把最新的内存表放在归并迭代器的最前面。这样存储引擎就能处理扫描请求。

## 本章检查点

引擎现在应能在可写和只读内存表上扫描任意请求范围。扫描返回有序、唯一且存活的键，并遵循与 `get` 相同的新者优先规则。

测试通过后，追踪一个出现在三个内存表中的键经过每一层迭代器的过程。准确指出它的旧版本在哪里被丢弃，以及它的墓碑（如果存在）在哪里对调用方不可见。同时确认 `next()` 返回 `Ok(())` 并不意味着迭代器仍然有效。

## 检验你的理解

### 正确性

* 如果某个键被删除（存在删除墓碑），你需要把它返回给用户吗？你在哪里处理了这个逻辑？
* 如果某个键有多个版本，用户会看到全部版本吗？你在哪里处理了这个逻辑？
* 如果你的键比较器无法给二叉堆提供稳定的顺序，会发生什么？
* 归并迭代器为什么必须按迭代器的构造顺序裁决重复键？
* 构造一个最小输入：如果 `MergeIterator::next` 只推进当前可见的子迭代器，而不推进所有位于该键上的子迭代器，就会产生重复键。

### Rust 与 API 设计

* 内存表迭代器为什么需要自引用结构体？
* 如果把自引用结构体换成内存表迭代器上的生命周期——例如 `MemTableIterator<'a>`，其中 `'a` 绑定到某个内存表或 `LsmStorageInner`——我们还能实现 `scan` 吗？
* 能否为 LSM 迭代器实现 Rust 风格的迭代器——例如带 `next(&mut self) -> Option<(Key, Value)>` 的那种？各有什么优缺点？
* 扫描接口类似 `fn scan(&self, lower: Bound<&[u8]>, upper: Bound<&[u8]>)`。要让接受 `key_a..key_b` 这类 Rust 范围语法，你可以怎么做？如果实现了这个 API，试试传入完整范围 `..`，观察会发生什么。
* 起始代码提供的归并迭代器接口保存的是 `Box<I>` 而不是 `I`。这可能是出于什么原因？

### 性能与并发行为

* 以输入迭代器的数量为参数，构建和推进你的归并迭代器的时间与空间复杂度分别是多少？
* 假设 (1) 你基于跳表内存表创建了迭代器，(2) 另一个线程向该内存表插入键。迭代器会看到这些新键吗？设计一个小实验，不要只依赖类型签名。

用[第一周周末自检](./week1-overview.md#end-of-week-self-check)校验核心不变量。其余设计问题可能有多个站得住脚的答案；在比较取舍之前，先说明你的工作负载和假设。你也可以在 Discord 社区讨论。

## 附加任务

* **前台迭代器。** 本课程假设所有操作都很短，因此迭代器可以持有内存表的引用。如果用户长时间持有一个迭代器，整个内存表——可能占用 256 MB——即使已经落盘到磁盘，也仍然留在内存中。为解决这个问题，请实现 `ForegroundIterator` 或 `LongIterator`，周期性地创建新的底层存储迭代器，让旧资源可以被回收。

{{#include copyright.md}}
