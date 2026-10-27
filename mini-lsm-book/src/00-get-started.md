<!--
  mini-lsm-book © 2022-2026 by Alex Chi Z is licensed under CC BY-NC-SA 4.0
-->

# 环境配置

起始代码与参考解都在 [Mini-LSM 仓库](https://github.com/skyzh/mini-lsm) 中。

## 安装 Rust

更多信息见 [https://rustup.rs](https://rustup.rs)。

## 克隆仓库

```
git clone https://github.com/skyzh/mini-lsm
```

## 打开起始代码

```
cd mini-lsm/mini-lsm-starter
code .
```

## 安装工具

仓库在 `rust-toolchain.toml` 中固定了所需的 Rust 工具链。如果你使用 `rustup`，Cargo 会自动选择并安装该工具链。

```
cargo x install-tools
```

## 检查起始代码并揭晓第一批测试

在仓库根目录下，先确认未改动的起始代码可以编译：

```
cargo check -p mini-lsm-starter --lib
```

这条命令应当通过。然后复制第 1 周第 1 天的测试，并记录初始的失败状态：

```
cargo x copy-test --week 1 --day 1
cargo x scheck
```

`cargo x scheck` 应当以状态码 1 退出，因为复制过来的六个第 1 天测试会在起始代码未实现的边界处失败。这些
失败是预期的起始证据，而不是环境配置失败。第 1 章会让这项检查通过。

现在你可以开始 [第一周：Mini-LSM](./week1-overview.md) 了。

{{#include copyright.md}}
