using System;
using System.Reflection;
using System.Threading;
using MinorShift.Emuera;
using uEmuera.Window;
using tEmuera;

namespace uEmuera
{
    public static class Application
    {
        internal static void EnableVisualStyles()
        {
        }

        internal static void SetCompatibleTextRenderingDefault(bool value)
        {
        }

        internal static void Run(MainWindow win)
        {
            if (!HeadlessOptions.ShowWarnings)
                typeof(Config).GetProperty("DisplayWarningLevel", BindingFlags.Public | BindingFlags.Static)
                    .SetValue(null, 4);
            UseExistingSaveDirectory();

            win.Init();
            win.FlushOutput();

            while (!win.ShouldExit)
            {
                Forms.Timer.Update();
                win.FlushOutput();

                if (win.NeedsInput)
                {
                    win.ShowInputPrompt();
                    var line = Console.ReadLine();
                    if (line == null)
                        break;
                    win.SubmitInput(line);
                    continue;
                }

                Thread.Sleep(10);
            }
        }

        private static void UseExistingSaveDirectory()
        {
            var savDir = MinorShift.Emuera.Program.ExeDir + "sav/";
            if (!System.IO.Directory.Exists(savDir))
                return;

            SetConfigProperty("UseSaveFolder", true);
            SetConfigProperty("SavDir", savDir);
            SetConfigProperty("ForceSavDir", savDir);
        }

        private static void SetConfigProperty(string name, object value)
        {
            typeof(Config).GetProperty(name, BindingFlags.Public | BindingFlags.Static)
                .SetValue(null, value);
        }
    }
}
