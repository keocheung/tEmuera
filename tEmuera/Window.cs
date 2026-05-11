using System;
using System.Collections.Generic;
using System.IO;
using System.Text;
using MinorShift.Emuera;
using MinorShift.Emuera.GameProc;
using MinorShift.Emuera.GameView;
using MinorShift._Library;
using uEmuera.Drawing;
using uEmuera.Forms;

namespace uEmuera.Window
{
    public class DebugDialog : IDisposable
    {
        public void Dispose()
        {
        }

        internal void SetParent(EmueraConsole console, Process process)
        {
        }

        internal void Show()
        {
        }

        internal void Focus()
        {
        }

        public bool Created { get { return true; } }
    }

    public class MainWindow : IDisposable
    {
        public static string uEmueraVer = "";

        private EmueraConsole console;
        private int printedLineCount;
        private bool created;
        private bool shouldExit;
        private bool promptShown;
        private int hiddenWarningCount;
        private bool skipWarningSourceLine;
        private int capturedButtonGeneration = -1;
        private int lastDisplayLineCount;
        private int temporaryLinesOnScreen;
        private int lastBackColorArgb;
        private bool terminalColorsApplied;
        private bool terminalBackgroundFilled;
        private readonly List<TerminalButton> currentButtons = new List<TerminalButton>();

        public void Dispose()
        {
            ResetTerminalStyle();
            if (console != null)
                console.Dispose();
        }

        public void Init()
        {
            if (created)
                return;

            created = true;
            console = new EmueraConsole(this);
            console.Initialize();
        }

        public void FlushOutput()
        {
            if (console == null)
                return;

            if (ApplyTerminalColors(false) || !terminalBackgroundFilled)
            {
                ClearTerminal();
                printedLineCount = 0;
                currentButtons.Clear();
                capturedButtonGeneration = -1;
            }

            var displayLineCount = console.GetDisplayLinesCount();
            if (displayLineCount < printedLineCount || displayLineCount < lastDisplayLineCount)
            {
                ClearTerminal();
                printedLineCount = 0;
                currentButtons.Clear();
                capturedButtonGeneration = -1;
            }

            while (printedLineCount < displayLineCount)
            {
                var line = console.GetDisplayLinesForuEmuera(printedLineCount++);
                if (line != null)
                {
                    CaptureButtons(line);
                    WriteLine(line);
                }
            }
            lastDisplayLineCount = displayLineCount;

            if (console.IsError)
                shouldExit = true;
        }

        public bool NeedsInput
        {
            get { return console != null && console.IsWaitingInput; }
        }

        public bool ShouldExit
        {
            get { return shouldExit; }
        }

        public void SubmitInput(string input)
        {
            if (console == null)
                return;

            promptShown = false;
            if (input == ":quit" || input == ":q")
            {
                ResetTerminalStyle();
                shouldExit = true;
                return;
            }
            if (input == ":help")
            {
                WriteStatusLine("[headless] 输入菜单编号或文本；空行用于 Enter/AnyKey；:choices 显示当前选项；:quit 退出；--show-warnings 显示加载警告。");
                return;
            }
            if (input == ":choices")
            {
                PrintChoices();
                return;
            }

            if (console.IsWaitingEnterKey)
                input = "";
            console.PressEnterKey(false, input ?? "", false);
            FlushOutput();
        }

        public void ShowInputPrompt()
        {
            if (promptShown || console == null)
                return;

            if (hiddenWarningCount > 0)
            {
                WriteStatusLine("[headless] hidden parser warnings: " + hiddenWarningCount + " (run with --show-warnings to display them)");
                hiddenWarningCount = 0;
            }

            ApplyTerminalColors(true);
            Console.Write(console.IsWaitingEnterKey ? "[enter] " : "[input] ");
            promptShown = true;
        }

        public void Refresh()
        {
            if (console != null)
                console.NeedSetTimer();
        }

        public void Close()
        {
            ResetTerminalStyle();
            shouldExit = true;
        }

        public void Focus()
        {
        }

        public void clear_richText()
        {
        }

        public void update_lastinput()
        {
        }

        internal void Reboot()
        {
            shouldExit = true;
        }

        internal void ShowConfigDialog()
        {
            WriteStatusLine("[headless] config dialog is not available");
        }

        public string InternalEmueraVer { get { return uEmueraVer; } }
        public string EmueraVerText { get { return uEmueraVer; } }
        public bool Created { get { return created; } }

        public ScrollBar ScrollBar = new ScrollBar();
        public PictureBox MainPicBox = new PictureBox();
        public string Text { get; set; }
        public ToolTip ToolTip = new ToolTip();
        public TextBox TextBox = new TextBox();
        public FormWindowState WindowState = FormWindowState.Normal;
        public Size ClientSize = new Size(800, 600);
        public Point Location = Point.Empty;

        private void WriteLine(ConsoleDisplayLine line)
        {
            var text = line.ToString();
            if (ShouldHideWarning(text))
                return;

            if (temporaryLinesOnScreen > 0)
                ClearTemporaryLines();

            Console.Write(RenderLine(line));
            Console.Write("\r\n");
            if (line.IsTemporary)
                temporaryLinesOnScreen = 1;
        }

        private bool ShouldHideWarning(string text)
        {
            if (tEmuera.HeadlessOptions.ShowWarnings)
                return false;

            if (skipWarningSourceLine)
            {
                skipWarningSourceLine = false;
                if (text.StartsWith("\t", StringComparison.Ordinal) || text.StartsWith(" ", StringComparison.Ordinal))
                    return true;
            }

            if (!text.StartsWith("警告Lv", StringComparison.Ordinal))
                return false;

            hiddenWarningCount += 1;
            skipWarningSourceLine = true;
            return true;
        }

        private string RenderLine(ConsoleDisplayLine line)
        {
            var builder = new StringBuilder();
            var cursorColumn = 0;
            AppendTerminalBaseStyle(builder, console != null ? console.bgColor : Config.BackColor);
            var buttons = line.Buttons;
            for (var i = 0; i < buttons.Length; i++)
            {
                var button = buttons[i];
                AppendButton(builder, button, ref cursorColumn);
            }
            AppendSpaces(builder, Math.Max(0, GetTerminalColumnCount() - cursorColumn));
            return builder.ToString();
        }

        private void AppendButton(StringBuilder builder, ConsoleButtonString button, ref int cursorColumn)
        {
            var parts = button.StrArray;
            for (var i = 0; i < parts.Length; i++)
            {
                var part = parts[i];
                var targetColumn = PixelToTerminalColumn(part.PointX);
                AppendSpaces(builder, Math.Max(0, targetColumn - cursorColumn));
                cursorColumn = Math.Max(cursorColumn, targetColumn);

                var text = part.ToString();
                AppendPartStyle(builder, part, false);
                AppendDisplayText(builder, text);
                builder.Append("\x1b[0m");
                AppendTerminalBaseStyle(builder, console != null ? console.bgColor : Config.BackColor);
                cursorColumn += GetEmueraTerminalWidth(text);
            }
        }

        private void ClearTerminal()
        {
            ApplyTerminalColors(true);
            FillTerminalBackground();
            terminalBackgroundFilled = true;
            temporaryLinesOnScreen = 0;
        }

        private void ClearTemporaryLines()
        {
            while (temporaryLinesOnScreen > 0)
            {
                Console.Write("\x1b[1A\r");
                WriteBlankTerminalLine();
                Console.Write("\r");
                temporaryLinesOnScreen--;
            }
        }

        private bool ApplyTerminalColors(bool force)
        {
            var backArgb = console != null ? console.bgColor.ToArgb() : Config.BackColor.ToArgb();
            var backgroundChanged = terminalColorsApplied && backArgb != lastBackColorArgb;
            if (!force && terminalColorsApplied && backArgb == lastBackColorArgb)
                return false;

            lastBackColorArgb = backArgb;
            terminalColorsApplied = true;
            AppendTerminalBaseStyle(Console.Out, console != null ? console.bgColor : Config.BackColor);
            AppendTerminalBaseStyle(Console.Error, console != null ? console.bgColor : Config.BackColor);
            return backgroundChanged;
        }

        private void WriteStatusLine(string text)
        {
            ApplyTerminalColors(true);
            var builder = new StringBuilder();
            AppendTerminalBaseStyle(builder, console != null ? console.bgColor : Config.BackColor);
            builder.Append(text);
            AppendSpaces(builder, Math.Max(0, GetTerminalColumnCount() - GetTerminalWidth(text)));
            Console.Write(builder.ToString());
            Console.Write("\r\n");
        }

        private void FillTerminalBackground()
        {
            var rows = GetTerminalRowCount();
            for (var row = 1; row <= rows; row++)
            {
                Console.Write("\x1b[" + row + ";1H");
                WriteBlankTerminalLine();
            }
            Console.Write("\x1b[H");
        }

        private static void WriteBlankTerminalLine()
        {
            Console.Write(new string(' ', GetTerminalColumnCount()));
        }

        private static void ResetTerminalStyle()
        {
            Console.Write("\x1b[0m");
        }

        private static void AppendPartStyle(StringBuilder builder, AConsoleDisplayPart part, bool isSelecting)
        {
            var color = Config.ForeColor;
            var fontStyle = FontStyle.Regular;

            var styled = part as ConsoleStyledString;
            if (styled != null)
            {
                var style = styled.StringStyle;
                color = isSelecting ? style.ButtonColor : style.Color;
                fontStyle = style.FontStyle;
            }
            else
            {
                var colored = part as AConsoleColoredPart;
                if (colored != null)
                    color = isSelecting ? colored.pButtonColor : colored.pColor;
            }

            AppendForegroundColor(builder, color);
            AppendFontStyle(builder, fontStyle);
        }

        private static void AppendTerminalBaseStyle(TextWriter writer, Color backColor)
        {
            writer.Write(GetBackgroundColorAnsi(backColor));
            writer.Write(GetForegroundColorAnsi(Config.ForeColor));
        }

        private static void AppendTerminalBaseStyle(StringBuilder builder, Color backColor)
        {
            builder.Append(GetBackgroundColorAnsi(backColor));
            builder.Append(GetForegroundColorAnsi(Config.ForeColor));
        }

        private static void AppendForegroundColor(StringBuilder builder, Color color)
        {
            builder.Append(GetForegroundColorAnsi(color));
        }

        private static string GetForegroundColorAnsi(Color color)
        {
            return "\x1b[38;2;" + color.R + ";" + color.G + ";" + color.B + "m";
        }

        private static string GetBackgroundColorAnsi(Color color)
        {
            return "\x1b[48;2;" + color.R + ";" + color.G + ";" + color.B + "m";
        }

        private static void AppendFontStyle(StringBuilder builder, FontStyle fontStyle)
        {
            if ((fontStyle & FontStyle.Bold) != 0)
                builder.Append("\x1b[1m");
            if ((fontStyle & FontStyle.Italic) != 0)
                builder.Append("\x1b[3m");
            if ((fontStyle & FontStyle.Underline) != 0)
                builder.Append("\x1b[4m");
            if ((fontStyle & FontStyle.Strikeout) != 0)
                builder.Append("\x1b[9m");
        }

        private static int PixelToTerminalColumn(int pointX)
        {
            var halfWidth = Math.Max(1, Config.FontSize / 2.0);
            return Math.Max(0, (int)Math.Round(pointX / halfWidth));
        }

        private static void AppendSpaces(StringBuilder builder, int count)
        {
            for (var i = 0; i < count; i++)
                builder.Append(' ');
        }

        private static int GetTerminalColumnCount()
        {
            try
            {
                return Math.Max(1, Console.WindowWidth);
            }
            catch
            {
                return 80;
            }
        }

        private static int GetTerminalRowCount()
        {
            try
            {
                return Math.Max(1, Console.WindowHeight);
            }
            catch
            {
                return 24;
            }
        }

        private static int GetTerminalWidth(string text)
        {
            var width = 0;
            for (var i = 0; i < text.Length; i++)
            {
                var codepoint = char.ConvertToUtf32(text, i);
                if (char.IsHighSurrogate(text[i]))
                    i++;
                width += GetTerminalWidth(codepoint);
            }
            return width;
        }

        private static int GetEmueraTerminalWidth(string text)
        {
            var width = 0;
            for (var i = 0; i < text.Length; i++)
            {
                var codepoint = char.ConvertToUtf32(text, i);
                if (char.IsHighSurrogate(text[i]))
                {
                    width += GetTerminalWidth(codepoint);
                    i++;
                    continue;
                }

                width += uEmuera.Utils.CheckHalfSize(text[i]) ? 1 : 2;
            }
            return width;
        }

        private static void AppendDisplayText(StringBuilder builder, string text)
        {
            for (var i = 0; i < text.Length; i++)
            {
                var codepoint = char.ConvertToUtf32(text, i);
                if (char.IsHighSurrogate(text[i]))
                {
                    builder.Append(text[i]);
                    i++;
                    builder.Append(text[i]);
                    continue;
                }

                var c = text[i];
                builder.Append(c);
                var emueraWidth = uEmuera.Utils.CheckHalfSize(c) ? 1 : 2;
                var terminalWidth = GetTerminalWidth(codepoint);
                if (terminalWidth < emueraWidth)
                    AppendSpaces(builder, emueraWidth - terminalWidth);
            }
        }

        private static int GetTerminalWidth(int codepoint)
        {
            if (codepoint == 0)
                return 0;
            if (codepoint < 32 || (codepoint >= 0x7f && codepoint < 0xa0))
                return 0;
            if (IsCombining(codepoint))
                return 0;
            if (IsWide(codepoint))
                return 2;
            return 1;
        }

        private static bool IsCombining(int codepoint)
        {
            return
                (codepoint >= 0x0300 && codepoint <= 0x036f) ||
                (codepoint >= 0x1ab0 && codepoint <= 0x1aff) ||
                (codepoint >= 0x1dc0 && codepoint <= 0x1dff) ||
                (codepoint >= 0x20d0 && codepoint <= 0x20ff) ||
                (codepoint >= 0xfe20 && codepoint <= 0xfe2f);
        }

        private static bool IsWide(int codepoint)
        {
            return
                codepoint == 0x3000 ||
                (codepoint >= 0x1100 && codepoint <= 0x115f) ||
                (codepoint >= 0x2329 && codepoint <= 0x232a) ||
                (codepoint >= 0x2e80 && codepoint <= 0xa4cf) ||
                (codepoint >= 0xac00 && codepoint <= 0xd7a3) ||
                (codepoint >= 0xf900 && codepoint <= 0xfaff) ||
                (codepoint >= 0xfe10 && codepoint <= 0xfe19) ||
                (codepoint >= 0xfe30 && codepoint <= 0xfe6f) ||
                (codepoint >= 0xff00 && codepoint <= 0xff60) ||
                (codepoint >= 0xffe0 && codepoint <= 0xffe6) ||
                (codepoint >= 0x1f300 && codepoint <= 0x1f64f) ||
                (codepoint >= 0x1f900 && codepoint <= 0x1f9ff);
        }

        private void CaptureButtons(ConsoleDisplayLine line)
        {
            if (capturedButtonGeneration != console.LastButtonGeneration)
            {
                capturedButtonGeneration = console.LastButtonGeneration;
                currentButtons.Clear();
            }

            var buttons = line.Buttons;
            for (var i = 0; i < buttons.Length; i++)
            {
                var button = buttons[i];
                if (!button.IsButton || button.Generation != console.LastButtonGeneration)
                    continue;
                if (HasButton(button.Inputs))
                    continue;
                currentButtons.Add(new TerminalButton(button.Inputs, button.ToString()));
            }
        }

        private bool HasButton(string input)
        {
            for (var i = 0; i < currentButtons.Count; i++)
            {
                if (currentButtons[i].Input == input)
                    return true;
            }
            return false;
        }

        private void PrintChoices()
        {
            if (currentButtons.Count == 0)
            {
                WriteStatusLine("[choices] no choices captured");
                return;
            }

            WriteStatusLine("[choices]");
            for (var i = 0; i < currentButtons.Count; i++)
                WriteStatusLine("  " + currentButtons[i].Input + "  " + currentButtons[i].Text);
        }

        private sealed class TerminalButton
        {
            public TerminalButton(string input, string text)
            {
                Input = input;
                Text = text;
            }

            public string Input { get; private set; }
            public string Text { get; private set; }
        }
    }
}
