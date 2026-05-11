# tEmuera

tEmuera 是一个在终端运行的，基于 [uEmuera](https://github.com/xerysherry/uEmuera) 代码的跨平台 Era 模拟器。

```text
tEmuera/HeadlessEntry.cs        命令行入口、游戏目录初始化
tEmuera/TerminalScreen.cs       终端输出、输入和 ANSI 样式渲染
tEmuera/HeadlessStubs.cs        headless 环境所需的资源与平台 stub
tEmuera/Application.cs          替代窗口应用循环
tEmuera/Window.cs               替代窗口对象
uEmuera/Assets/Scripts/Emuera/  Emuera 核心脚本、数据、执行器和显示逻辑
uEmuera/Assets/Scripts/uEmuera/ 兼容层与必要平台抽象
```

## 运行方式

需要安装 .NET SDK。当前项目目标框架为 `net10.0`。

```bash
dotnet run --project tEmuera -- /path/to/era-game
```

默认会隐藏加载期脚本 warning，方便直接游玩。需要排查脚本加载问题时可加`--show-warnings`参数：

```bash
dotnet run --project tEmuera -- --show-warnings /path/to/era-game
```

运行中可输入菜单编号或文本；空行用于 Enter/AnyKey。内置命令：

```text
:help      显示终端命令
:choices   查看当前捕获到的输入选项
:quit      退出
```

## 打包为本地 .NET tool

```bash
dotnet pack tEmuera
dotnet tool install --global --add-source artifacts/packages tEmuera
temuera /path/to/era-game
```

如果在 macOS/Homebrew 安装的 .NET 下运行 tool 时提示 `You must install .NET to run this application`，通常是 .NET apphost 没找到 Homebrew 的 runtime 路径。可在 shell 配置中加入：

```bash
export DOTNET_ROOT=/opt/homebrew/opt/dotnet/libexec
export DOTNET_ROOT_ARM64=/opt/homebrew/opt/dotnet/libexec
```

## 当前能力

已支持：

1. 从游戏目录加载 `ERB/CSV/DAT/CONFIG/SAV` 等资源并执行脚本主流程
2. 纯文本输出、按钮文本捕获、数字输入、字符串输入、Enter/AnyKey
3. 读取游戏目录下已有 `sav/` 存档，并将存档目录固定到游戏目录内
4. 在 macOS/Unix 上创建临时大小写兼容 overlay，为资源目录和文件建立大小写别名
5. 使用 ANSI truecolor 渲染文本颜色、背景色和 bold/italic/underline/strikeout 样式
6. 使用 Emuera 字符宽度规则补偿终端宽度差异，改善地图字符和全角符号对齐
7. 对 `ClearDisplay` 做终端清屏并从当前显示列表重绘
8. 对 temporary line 做上移清行替换，减少 loading 行堆积

## 当前限制

tEmuera 目前仍是终端运行器，不是完整 GUI 等价实现：

1. 没有鼠标 hover/点击，按钮状态默认按未选中样式渲染
2. 图像、CBG/GXX、调试窗口、配置窗口等 GUI 功能仍为 stub 或 no-op
3. HTML/富文本显示只覆盖文本颜色和基础字体样式，未完整模拟 uEmuera 的全部绘制细节
4. 终端字符宽度依赖具体终端字体和 East Asian Width 策略，地图类输出可能仍需针对终端环境微调
5. `Properties.Resources` 目前由 `HeadlessStubs.cs` 提供必要字符串 stub，尚未完整接入原始资源管理器

## 与 uEmuera 的关系

uEmuera 是 Emuera 的 Unity3D 移植版本，面向 Android 等非 Windows 平台。tEmuera 直接复用其中的 Emuera 核心代码，并替换 Unity UI、窗口、资源和平台层，使其可以在普通终端中运行。

console 项目中纳入编译的参考代码：

```text
uEmuera/Assets/Scripts/Emuera/**
uEmuera/Assets/Scripts/uEmuera/Forms.cs
uEmuera/Assets/Scripts/uEmuera/Utils.cs
uEmuera/Assets/Scripts/uEmuera/Drawing.cs
uEmuera/Assets/Scripts/uEmuera/Media.cs
uEmuera/Assets/Scripts/uEmuera/VisualBasic.cs
uEmuera/Assets/Scripts/uEmuera/partial/**
```

Unity UI 相关脚本不会进入 tEmuera 编译：

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
