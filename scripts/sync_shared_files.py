#!/usr/bin/env python3
"""在 Windows 上同步上游用符号链接共享的源文件。

上游仓库用 git 符号链接在 `mini-lsm` / `mini-lsm-mvcc` / `mini-lsm-starter` 之间共享
部分源文件（测试、CLI、debug 工具等）。Windows 默认 `core.symlinks=false`，clone 下来
会得到一堆「内容是一行路径」的普通文件，无法编译。

本项目把符号链接展开成了真实文件，因此改动共享文件后需要重新同步一次：

    python scripts/sync_shared_files.py          # 从规范副本复制到各 crate
    python scripts/sync_shared_files.py --check  # 只检查是否已同步（CI 可用）

规范副本（canonical）的选取与上游一致：
  * `mini-lsm-starter/src` 是 CLI、debug、mvcc 相关共享文件的正本；
  * `mini-lsm/src/tests` 是 week1/week2 测试用例的正本。
"""

from __future__ import annotations

import filecmp
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# (规范副本, 需要同步的副本)
SHARED_FILES: list[tuple[str, str]] = [
    # CLI 与 debug 工具：mini-lsm-starter 为正本
    ("mini-lsm-starter/src/bin/compaction-simulator.rs", "mini-lsm/src/bin/compaction-simulator.rs"),
    ("mini-lsm-starter/src/bin/mini-lsm-cli.rs", "mini-lsm/src/bin/mini-lsm-cli.rs"),
    ("mini-lsm-starter/src/bin/compaction-simulator.rs", "mini-lsm-mvcc/src/bin/compaction-simulator.rs"),
    ("mini-lsm-starter/src/bin/mini-lsm-cli.rs", "mini-lsm-mvcc/src/bin/mini-lsm-cli.rs"),
    ("mini-lsm-starter/src/debug.rs", "mini-lsm/src/debug.rs"),
    ("mini-lsm-starter/src/debug.rs", "mini-lsm-mvcc/src/debug.rs"),
    # MVCC 骨架：mini-lsm-starter 为正本
    ("mini-lsm-starter/src/mvcc.rs", "mini-lsm/src/mvcc.rs"),
    ("mini-lsm-starter/src/mvcc/txn.rs", "mini-lsm/src/mvcc/txn.rs"),
    ("mini-lsm-starter/src/mvcc/watermark.rs", "mini-lsm/src/mvcc/watermark.rs"),
]

# week1/week2 测试：mini-lsm/src/tests 为正本，同步到 mini-lsm-mvcc/src/tests
for _test in [
    "harness.rs",
    "week1_day1.rs",
    "week1_day2.rs",
    "week1_day3.rs",
    "week1_day4.rs",
    "week1_day5.rs",
    "week1_day6.rs",
    "week1_day7.rs",
    "week2_day1.rs",
    "week2_day2.rs",
    "week2_day3.rs",
    "week2_day4.rs",
    "week2_day5.rs",
    "week2_day6.rs",
]:
    SHARED_FILES.append(
        (f"mini-lsm/src/tests/{_test}", f"mini-lsm-mvcc/src/tests/{_test}")
    )


def main() -> int:
    check_only = "--check" in sys.argv[1:]
    dirty: list[str] = []

    for src_rel, dst_rel in SHARED_FILES:
        src = ROOT / src_rel
        dst = ROOT / dst_rel
        if not src.is_file():
            print(f"[跳过] 规范副本不存在：{src_rel}")
            continue
        if dst.is_file() and filecmp.cmp(src, dst, shallow=False):
            continue
        dirty.append(dst_rel)
        if check_only:
            continue
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(src, dst)
        print(f"[同步] {src_rel} -> {dst_rel}")

    if check_only:
        if dirty:
            print("\n以下文件与规范副本不一致，请运行 python scripts/sync_shared_files.py：")
            for item in dirty:
                print(f"  - {item}")
            return 1
        print("所有共享文件均已同步。")
        return 0

    print("同步完成。" if dirty else "所有共享文件均已同步，无需改动。")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
