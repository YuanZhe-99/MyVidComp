# 函数索引

`src/` 中已记录的声明总数：**368**。

源码中每个声明都带有 `AI-FUNC-SUMMARY` 注释，其中每一条都恰好出现在下面某一个页面上。与模块一一
对应的页面就是那个模块；拆分 `src/lib.rs` 的那些页面按代码所属的流水线阶段分组，因为流水线本身就
在同一个文件里。

| 页面 | 覆盖范围 | 声明数 |
|---|---|---|
| [options.md](options.md) | `src/options.rs` | 29 |
| [codecs.md](codecs.md) | `src/codec.rs` | 12 |
| [quality.md](quality.md) | `src/vmaf.rs`、`src/search.rs` | 24 |
| [review.md](review.md) | `src/review.rs` | 20 |
| [ffi-abi.md](ffi-abi.md) | `src/ffi.rs` | 19 |
| [workflow-events.md](workflow-events.md) | 运行过程、事件、进度与提交 | 79 |
| [cli-and-config.md](cli-and-config.md) | 命令行与设置文件 | 28 |
| [encoders-quality.md](encoders-quality.md) | 编码器检测与画质估算 | 49 |
| [discovery-probing.md](discovery-probing.md) | 查找并探测文件 | 20 |
| [transcoding.md](transcoding.md) | 计划并运行编码 | 55 |
| [validation-commit.md](validation-commit.md) | 检查完成的文件 | 17 |
| [stream-policy.md](stream-policy.md) | 流映射与容器选择 | 11 |
| [chapter-carrier-parsing.md](chapter-carrier-parsing.md) | ISO BMFF 章节证据 | 5 |

新增、删除或修改一个声明，就要在同一次改动中更新它所在的页面、旁边的计数、上面的总数，以及英文
原始版本。`scripts/check-doc-parity.ps1` 会检查两个语言树是否仍然对得上。
