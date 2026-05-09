# tEmuera

本目录用于探索将 Emuera/uEmuera 的核心逻辑移植为 Rust 终端运行器的可行性。`uEmuera/` 是参考工程，不作为当前探索文档和实验代码的主目录。

## Rust终端移植 / C# Headless验证路线

目标是先验证 uEmuera 核心逻辑能否脱离 Unity 图形界面运行，再逐步评估用 Rust 实现终端版的成本。建议不要一开始就直接全量 Rust 重写，而是先做一个 C# headless console harness。

### 为什么先做 C# headless

当前参考工程是 Unity 项目，没有独立的 `.csproj`。Emuera 核心代码主要位于 `uEmuera/Assets/Scripts/Emuera`，Unity 适配层位于 `uEmuera/Assets/Scripts/uEmuera` 和外层 Unity UI 脚本。脚本执行核心 `Process` 仍直接依赖 `EmueraConsole`，而 `EmueraConsole` 同时承担文本输出、按钮输入、计时器、窗口状态、图像和刷新等职责。

先做 C# headless 可以在不改变脚本执行语义的前提下，验证纯文本输入输出、ERB 加载、变量、表达式、存档等核心行为。这样后续 Rust 移植可以用 C# headless 版本作为行为基准。

### 推荐运行方式

使用 `.NET 8 SDK` 或 `.NET 6 SDK` 创建独立 console 项目，不使用 Unity 运行验证：

```bash
dotnet new console -n UEmuera.Headless
dotnet run --project UEmuera.Headless -- /path/to/era-game
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
