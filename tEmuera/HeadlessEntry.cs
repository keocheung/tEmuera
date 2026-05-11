using System;
using System.IO;
using System.Reflection;
using System.Security.Cryptography;
using System.Text;
using MinorShift._Library;
using MinorShift.Emuera;

namespace tEmuera
{
    public static class HeadlessOptions
    {
        public static bool ShowWarnings { get; set; }
    }

    public static class HeadlessEntry
    {
        public static void Main(string[] args)
        {
            var gamePath = ParseArgs(args);
            if (gamePath == null || !Directory.Exists(gamePath))
            {
                Console.Error.WriteLine("Usage: temuera [--show-warnings] /path/to/era-game");
                Environment.ExitCode = 2;
                return;
            }

            var gameDir = Path.GetFullPath(gamePath);
            var runtimeGameDir = PrepareCaseInsensitiveOverlay(gameDir);
            Sys.SetWorkFolder(Path.GetDirectoryName(runtimeGameDir) ?? Directory.GetCurrentDirectory());
            Sys.SetSourceFolder(Path.GetFileName(runtimeGameDir));
            typeof(Sys).GetProperty("ExeDir", BindingFlags.Public | BindingFlags.Static)
                .SetValue(null, EnsureTrailingSeparator(runtimeGameDir));
            uEmuera.Logger.info = null;
            uEmuera.Logger.warn = value => Console.Error.WriteLine(value);
            uEmuera.Logger.error = value => Console.Error.WriteLine(value);

            using (TerminalScreen.Enter())
                Program.Main(Array.Empty<string>());
        }

        private static string ParseArgs(string[] args)
        {
            string gamePath = null;
            for (var i = 0; i < args.Length; i++)
            {
                if (args[i] == "--show-warnings")
                {
                    HeadlessOptions.ShowWarnings = true;
                    continue;
                }

                if (gamePath != null)
                    return null;
                gamePath = args[i];
            }

            return gamePath;
        }

        private static string EnsureTrailingSeparator(string path)
        {
            if (path.EndsWith(Path.DirectorySeparatorChar.ToString(), StringComparison.Ordinal))
                return path;
            return path + Path.DirectorySeparatorChar;
        }

        private static string PrepareCaseInsensitiveOverlay(string sourceRoot)
        {
            var overlayRoot = Path.Combine(Path.GetTempPath(), "tEmuera", HashPath(sourceRoot));
            if (Directory.Exists(overlayRoot))
                Directory.Delete(overlayRoot, true);
            Directory.CreateDirectory(overlayRoot);

            LinkDirectoryEntries(sourceRoot, overlayRoot, false);

            foreach (var dirName in new[] { "CSV", "ERB", "DAT", "DEBUG", "RESOURCES", "resources" })
            {
                var sourceDir = FindChildDirectory(sourceRoot, dirName);
                if (sourceDir == null)
                    continue;
                var overlayDir = Path.Combine(overlayRoot, dirName);
                if (Directory.Exists(overlayDir))
                {
                    var attributes = File.GetAttributes(overlayDir);
                    if ((attributes & FileAttributes.ReparsePoint) != 0)
                        Directory.Delete(overlayDir);
                    else
                        continue;
                }
                Directory.CreateDirectory(overlayDir);
                LinkDirectoryEntries(sourceDir, overlayDir, true);
            }

            return overlayRoot;
        }

        private static void LinkDirectoryEntries(string sourceDir, string overlayDir, bool recursive)
        {
            foreach (var directory in Directory.GetDirectories(sourceDir))
            {
                var name = Path.GetFileName(directory);
                var target = Path.Combine(overlayDir, name);
                Directory.CreateSymbolicLink(target, directory);
                AddAliasLink(overlayDir, name, directory, true);

                if (recursive)
                {
                    var realDir = target;
                    if (Directory.Exists(realDir))
                    {
                        Directory.Delete(realDir);
                        Directory.CreateDirectory(realDir);
                        LinkDirectoryEntries(directory, realDir, true);
                    }
                }
            }

            foreach (var file in Directory.GetFiles(sourceDir))
            {
                var name = Path.GetFileName(file);
                File.CreateSymbolicLink(Path.Combine(overlayDir, name), file);
                AddAliasLink(overlayDir, name, file, false);
            }
        }

        private static void AddAliasLink(string overlayDir, string name, string sourcePath, bool isDirectory)
        {
            foreach (var alias in new[] { name.ToUpperInvariant(), name.ToLowerInvariant() })
            {
                var aliasPath = Path.Combine(overlayDir, alias);
                if (File.Exists(aliasPath) || Directory.Exists(aliasPath))
                    continue;
                if (isDirectory)
                    Directory.CreateSymbolicLink(aliasPath, sourcePath);
                else
                    File.CreateSymbolicLink(aliasPath, sourcePath);
            }
        }

        private static string FindChildDirectory(string root, string childName)
        {
            foreach (var directory in Directory.GetDirectories(root))
            {
                if (string.Equals(Path.GetFileName(directory), childName, StringComparison.OrdinalIgnoreCase))
                    return directory;
            }
            return null;
        }

        private static string HashPath(string path)
        {
            using (var sha = SHA256.Create())
            {
                var bytes = sha.ComputeHash(Encoding.UTF8.GetBytes(path));
                var builder = new StringBuilder(16);
                for (var i = 0; i < 8; i++)
                    builder.AppendFormat("{0:x2}", bytes[i]);
                return builder.ToString();
            }
        }
    }
}
