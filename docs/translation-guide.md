# Mini-LSM 中文化规范

本仓库是 [skyzh/mini-lsm](https://github.com/skyzh/mini-lsm) 的中文化分支，用作课程阶段 Ⅰ
的项目底座。为保证多人协作时译文风格一致，所有中文化改动都遵循本规范。

## 一、绝对不改动的内容

1. **代码 token**：标识符、类型、函数名、宏、属性、字符串字面量、数字。
   - 错误信息字符串（`anyhow!("...")`、`ensure!(cond, "...")`、`bail!`) **一律保留英文原文**，
     因为测试与快照可能比对输出。
2. **Markdown 结构**：标题层级、列表缩进、表格列、代码围栏（```）、图片路径、链接 URL。
3. **mdBook 指令**：`{{#include copyright.md}}` 等保持原样。
4. **HTML 块**：`<div class="warning">`、`<img>`、`<details>` 等标签与 `class` 属性。
5. **文件头许可证声明**：`// Copyright (c) ...` 与 Apache 2.0 许可正文保持英文。
6. **`// ANCHOR:` / `// ANCHOR_END:` 行**（若出现）：锚点名用于 mdBook 引用，禁止翻译。

## 二、注释翻译规则

- 翻译 `///`、`//!`、`//`、`/* */` 中的**说明性文字**。
- 形如 `// TODO(you): remove this lint ...` 的练习提示要翻译说明部分，保留 `TODO(you)` 标记。
- 代码块内注释可翻译；但代码块里的**期望输出**（doctest 的 `assert_eq!` 内容、示例输出行）不动。
- 行内注释与代码同一行时，保持缩进与 `//` 位置不变。
- 注释里的 API 名、类型名、路径（如 `src/lsm_storage.rs`）、章节名（`Week 2 Day 6`）保留英文。
- 译文不要换行重排：原文一行注释尽量对译成一行，避免整段重写导致 diff 巨大。

## 三、术语表（统一用法）

| 英文 | 统一译法 | 说明 |
| --- | --- | --- |
| memtable | 内存表（memtable） | 首次出现给中英对照，之后用「内存表」 |
| immutable memtable | 只读内存表 | |
| mutable memtable | 可写内存表 | |
| SSTable / SST | SSTable | 不译，作为专有名词 |
| block | 块（block） | |
| compaction | 压缩（compaction） | 动词用「压缩」 |
| level / tier | 层（level）/ 层（tier） | |
| manifest | 清单（manifest） | |
| WAL / write-ahead log | 预写日志（WAL） | |
| flush | 落盘 | 内存表写盘 |
| freeze | 冻结 | 内存表转为只读 |
| tombstone | 墓碑（tombstone） | 删除标记 |
| iterator / merge iterator | 迭代器 / 归并迭代器 | |
| snapshot | 快照 | |
| watermark | 水位线 | |
| timestamp / ts | 时间戳 / ts | |
| transaction | 事务 | |
| commit | 提交 | |
| isolation | 隔离级别 / 隔离性 | |
| serializable | 可串行化 | |
| optimistic concurrency control | 乐观并发控制（OCC） | |
| bloom filter | 布隆过滤器 | |
| checksum | 校验和 | |
| prefix key encoding | 前缀键编码 | |
| block cache | 块缓存 | |
| key / value | 键 / 值 | |
| seek | 定位 / 查找 | 视上下文 |
| point lookup | 点查 | |
| range scan | 范围扫描 | |
| invariant | 不变量 | |
| refactor | 重构 | |
| unsafe block | `unsafe` 代码块 | 不译代码关键字 |

## 四、行文风格

- 面向大二学生：句子短、先结论后原因、术语首现给中英对照。
- 不使用比喻和口语化类比，直接讲机制与后果。
- 中文与英文、数字之间加一个半角空格（如「使用 `Bytes` 存储」）。
- 使用中文全角标点（，。：；？！）,括号用全角（），代码与术语周围保留半角。
- 第二人称用「你」，不用「您」。
- 不增写原文没有的内容；如需补充教学说明，用引用块 `> **补充：**` 明确标记为译者补充。

## 五、校验方式

```bash
cargo check --workspace     # 注释改动不得影响编译
cargo test -p mini-lsm      # 参考实现测试应全部通过
```
