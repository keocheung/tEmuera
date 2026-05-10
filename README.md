# tEmuera

本目录用于探索将 Emuera/uEmuera 的核心逻辑移植为 Rust 终端运行器的可行性。`uEmuera/` 是参考工程，不作为当前探索文档和实验代码的主目录。

## Rust终端移植 / C# Headless验证路线

目标是先验证 uEmuera 核心逻辑能否脱离 Unity 图形界面运行，再逐步评估用 Rust 实现终端版的成本。建议不要一开始就直接全量 Rust 重写，而是先做一个 C# headless console harness。

### 为什么先做 C# headless

当前参考工程是 Unity 项目，没有独立的 `.csproj`。Emuera 核心代码主要位于 `uEmuera/Assets/Scripts/Emuera`，Unity 适配层位于 `uEmuera/Assets/Scripts/uEmuera` 和外层 Unity UI 脚本。脚本执行核心 `Process` 仍直接依赖 `EmueraConsole`，而 `EmueraConsole` 同时承担文本输出、按钮输入、计时器、窗口状态、图像和刷新等职责。

先做 C# headless 可以在不改变脚本执行语义的前提下，验证纯文本输入输出、ERB 加载、变量、表达式、存档等核心行为。这样后续 Rust 移植可以用 C# headless 版本作为行为基准。

### 推荐运行方式

使用 .NET SDK 创建独立 console 项目，不使用 Unity 运行验证：

```bash
dotnet new console -n UEmuera.Headless
dotnet run --project UEmuera.Headless -- /path/to/era-game
```

当前仓库已包含 `UEmuera.Headless/`，项目目标框架为 `net9.0`。可直接运行：

```bash
dotnet run --project UEmuera.Headless -- "games/era紅魔館protoNTR0036甜艮菜魔改版整合升级V2.08 (36旧版-附Debug)"
```

默认会隐藏加载期脚本警告以便终端游玩；需要排查脚本加载问题时加 `--show-warnings`。运行中输入菜单编号或文本，空行用于 Enter/AnyKey，`:help` 显示 headless 命令，`:quit` 退出。

### 当前 C# Headless 状态

`UEmuera.Headless/` 目前已经可以作为 uEmuera 核心逻辑的终端验证器使用。它复用 `uEmuera/Assets/Scripts/Emuera` 与必要的 `uEmuera/Assets/Scripts/uEmuera` 代码，替换 Unity 窗口、图形、资源和平台层。

已完成能力：

1. 从游戏目录加载 `ERB/CSV/DAT/CONFIG/SAV` 等资源，执行脚本主流程
2. 支持纯文本输出、按钮文本捕获、数字/字符串输入、Enter/AnyKey
3. 支持 `:choices` 查看当前捕获到的输入选项，默认不重复打印一套选择列表
4. 支持读取已有 `sav/` 存档目录，并将存档目录固定到游戏目录下
5. 在 macOS/Unix 上创建临时大小写兼容 overlay，为资源目录和文件建立大小写别名
6. 默认隐藏加载期 warning，`--show-warnings` 可恢复显示
7. 使用现代终端 ANSI truecolor 渲染 uEmuera 文本颜色、背景色和 bold/italic/underline/strikeout 样式
8. 使用 Emuera 字符宽度规则补偿终端宽度差异，改善 `■`、`┃`、`＜＞` 等地图字符的对齐
9. 对 `ClearDisplay` 做终端清屏并从当前显示列表重绘；对 temporary line 做上移清行替换，减少 loading 行堆积

目前的终端行为仍不是完整 GUI 等价：

1. 没有鼠标 hover/点击，因此按钮 `FocusColor/ButtonColor` 只保留接口路径，默认按未选中状态渲染
2. 图像、CBG/GXX、调试窗口、配置窗口等 GUI 功能仍为 stub 或 no-op
3. HTML/富文本显示只覆盖文本颜色和基础字体样式，未完整模拟 uEmuera 的全部绘制细节
4. 终端字符宽度依赖具体终端字体和 East Asian Width 策略，地图类输出仍可能需要针对终端环境微调
5. `Properties.Resources` 目前由 `HeadlessStubs.cs` 提供必要字符串 stub，尚未完整接入原始资源管理器

### Rust Headless 重写

仓库现在包含一个 Rust workspace，当前 crate 为 `crates/temuera-headless`。Rust 版不是 C# `UEmuera.Headless` 的逐行翻译，而是按 Rust 社区习惯拆成小模块：

```text
crates/temuera-headless/src/
  app.rs         运行器主流程和交互命令
  cli.rs         参数解析
  config.rs      emuera.config 读取
  csv.rs         CSV 读取和常量名表
  error.rs       统一 Result/Error 类型
  expr.rs        表达式 parser/evaluator 骨架
  fs_overlay.rs  大小写兼容临时 overlay
  game.rs        游戏目录校验、目录布局和资源加载
  runtime.rs     运行时变量地址和值存储骨架
  script.rs      ERB/ERH 源文件加载和函数索引
  terminal.rs    ANSI truecolor、清屏、文本宽度工具
```

运行方式：

```bash
cargo run -p temuera-headless -- "games/era紅魔館protoNTR0036甜艮菜魔改版整合升级V2.08 (36旧版-附Debug)"
```

可用参数：

```bash
cargo run -p temuera-headless -- --show-warnings /path/to/era-game
cargo run -p temuera-headless -- --no-overlay /path/to/era-game
```

当前 Rust 版已完成：

1. CLI 和帮助信息
2. 游戏目录校验
3. 与 C# Headless 同思路的大小写兼容 overlay，`CSV/ERB/DAT/DEBUG/RESOURCES/resources` 会被实体化并递归建立别名
4. 资源扫描，统计 `ERB/ERH/CSV/DAT/SAV`、`emuera.config` 和 `sav/`
5. `emuera.config` 键值读取
6. CSV 文件读取、基础 CSV 行解析、从 `Talent.csv` 等表构建 `性別 -> 2` 这类常量名表
7. ERB/ERH 源文件读取、行分类、函数和标签索引
8. 运行时变量地址和值存储骨架，支持 `TALENT:ARG:性別` 这类地址解析
9. 表达式 parser/evaluator 骨架，支持整数/字符串、变量读取、算术、比较、逻辑、位运算和 `? #` 条件表达式
10. 最小 ERB 指令 parser，覆盖 `PRINT/PRINTL/PRINTS/PRINTSL/PRINTFORM/PRINTFORML/DRAWLINE/BAR`、赋值、`++/--`、`SIF/IF/ELSEIF/ELSE/ENDIF`、`FOR/NEXT`、`GOTO`、`CALL`、`INPUT/INPUTS`
11. 最小 VM，可顺序执行函数、递归 `CALL`、处理条件、循环、输入队列和基础 FORM 展开
12. ANSI truecolor 前景/背景、清屏、reset 和基础文本样式输出
13. Emuera 字符宽度工具，用于后续终端布局
14. bootstrap 交互命令：`:help`、`:scan`、`:paths`、`:functions`、`:find NAME`、`:parse NAME`、`:run NAME [STEPS]`、`:run-with NAME INPUT...`、`:name TABLE NAME`、`:var ADDRESS[=VALUE]`、`:eval EXPR`、`:clear`、`:quit`

当前 Rust 版尚未实现完整 ERB 指令 parser、控制流执行器、函数调用栈、角色数据初始化和存档格式，所以还不能替代 C# Headless 运行游戏。现阶段定位是把平台/终端/文件系统边界、源加载、名字解析、变量地址和表达式基础先迁到 Rust，后续再按模块迁移 VM。

当前测试游戏进度：

```bash
cargo run -q -p temuera-headless -- "games/era紅魔館protoNTR0036甜艮菜魔改版整合升级V2.08 (36旧版-附Debug)"
```

在交互中执行 `:run EVENTFIRST 500` 可以跑到首个模式选择 `INPUT`。执行 `:run-with EVENTFIRST 0 0 你 你` 可以越过模式选择、角色名称和 CALLNAME 输入，进入角色状态显示；目前会在后续大量状态显示/未完整 FORM 与 VM 行为处触发 step limit。

验证命令：

```bash
cargo fmt --all
cargo check -p temuera-headless
cargo test -p temuera-headless
```

console 项目中纳入编译的参考代码：

```text
uEmuera/Assets/Scripts/Emuera/**
uEmuera/Assets/Scripts/uEmuera/**
```

需要排除或替换 Unity UI 相关脚本：

```text
uEmuera/Assets/Scripts/EmueraMain.cs
uEmuera/Assets/Scripts/EmueraThread.cs
uEmuera/Assets/Scripts/EmueraBehaviour.cs
uEmuera/Assets/Scripts/EmueraContent.cs
uEmuera/Assets/Scripts/EmueraLine.cs
uEmuera/Assets/Scripts/EmueraImage.cs
uEmuera/Assets/Scripts/SpriteManager.cs
uEmuera/Assets/Scripts/Inputpad.cs
uEmuera/Assets/Scripts/OptionWindow.cs
uEmuera/Assets/Scripts/QuickButtons.cs
uEmuera/Assets/Scripts/Scalepad.cs
uEmuera/Assets/Scripts/FirstWindow.cs
uEmuera/Assets/Scripts/MainEntry.cs
uEmuera/Assets/Scripts/GenericUtils.cs
```

### Headless最小实现

第一阶段可以继续复用 `EmueraConsole`，但将窗口和平台层替换为 headless 实现。最小需要实现：

1. 将新增显示行输出到 stdout
2. 从 stdin 读取 `INPUT`、`INPUTS`、Enter/AnyKey
3. 调用 `console.PressEnterKey(...)` 把输入送回执行器
4. 将窗口标题、焦点、刷新、MessageBox 等行为实现为 no-op 或 stderr 输出
5. 用简单循环或 `Task.Delay` 驱动 Timer

可以先让 `Application.Run(win)` 变成阻塞循环：初始化 `MainWindow`，持续刷新输出，等待 stdin 输入，输入后继续推进脚本。

### Rust终端版分阶段目标

1. MVP 终端版：支持 `csv/erb/config/sav`、纯文本输出、按钮文本选择、`INPUT/INPUTS`
2. 兼容型终端版：支持颜色、样式、滚动日志、超时输入、宏、错误定位和存档兼容
3. 完整 Rust 引擎：重写 lexer/parser、表达式、变量系统、执行器、资源系统和存档格式

Rust 侧建议使用 `crossterm` 或 `ratatui` 实现终端交互。图像、鼠标、GXX/CBG、调试窗口等功能应放到后续阶段，第一阶段只验证文字游戏核心流程。

### 验证策略

建立一组最小 era 脚本作为 golden tests，覆盖：

1. ERB 加载和标签跳转
2. 变量读写和数组
3. 表达式、FORM、RAND
4. `CALL`、`JUMP`、系统过程
5. `INPUT`、`INPUTS`、按钮输入
6. 保存和读取

Rust 实现完成某个模块后，用 C# headless 版本的输出和状态作为基准进行对比。
