using System;

namespace tEmuera
{
    internal sealed class TerminalScreen : IDisposable
    {
        private readonly bool enabled;
        private bool disposed;

        private TerminalScreen(bool enabled)
        {
            this.enabled = enabled;
        }

        public static TerminalScreen Enter()
        {
            if (Console.IsOutputRedirected)
                return new TerminalScreen(false);

            Console.Write("\x1b[?1049h\x1b[?25l\x1b[2J\x1b[H");
            return new TerminalScreen(true);
        }

        public void Dispose()
        {
            if (disposed)
                return;

            disposed = true;
            if (!enabled)
                return;

            Console.Write("\x1b[0m\x1b[?25h\x1b[?1049l");
        }
    }
}
