using System;
using System.Collections.Generic;
using System.IO;
using System.Security.Cryptography;
using System.Text;

public static class GenericUtils
{
    public static string GetFilename(string path)
    {
        return Path.GetFileName(path);
    }

    public static List<string> CalcMd5ListForConfig(byte[] data)
    {
        var md5s = new List<string>();
        var start = 0;
        var count = 0;

        while (start < data.Length && data[start] != 0)
        {
            while (start + count < data.Length && data[start + count] != ':')
                count += 1;
            md5s.Add(CalcMd5(data, start, count));

            start += count;
            count = 0;

            while (start < data.Length && data[start] != '\r' && data[start] != '\n')
                start += 1;
            while (start < data.Length && (data[start] == '\r' || data[start] == '\n'))
                start += 1;
        }

        return md5s;
    }

    private static string CalcMd5(byte[] data, int offset, int count)
    {
        using (var md5 = MD5.Create())
        {
            var hash = md5.ComputeHash(data, offset, count);
            var builder = new StringBuilder(hash.Length * 2);
            for (var i = 0; i < hash.Length; i++)
                builder.AppendFormat("{0:X2}", hash[i]);
            return builder.ToString();
        }
    }
}

public static class SpriteManager
{
    public sealed class TextureInfo
    {
        public UnityEngine.Texture2D texture = new UnityEngine.Texture2D();
        public int width { get { return texture.width; } }
        public int height { get { return texture.height; } }
    }

    public sealed class TextureInfoOtherThread
    {
        public System.Threading.Mutex mutex = new System.Threading.Mutex();
    }

    public static TextureInfo GetTextureInfo(string name, string path)
    {
        return new TextureInfo();
    }

    public static TextureInfoOtherThread GetTextureInfoOtherThread(string name, string path, Action<TextureInfo> callback)
    {
        var result = new TextureInfoOtherThread();
        callback(new TextureInfo());
        return result;
    }

    public static string[] GetResourceCSVLines(string filename)
    {
        return null;
    }

    public static void SetResourceCSVLine(string filename, string[] lines)
    {
    }
}

namespace Properties
{
    public static class Resources
    {
        public const string SyntaxErrMesMethodDefaultArgumentNum0 = "関数{0}の引数の数が正しくありません";
        public const string SyntaxErrMesMethodDefaultArgumentNum1 = "関数{0}の引数の数が正しくありません";
        public const string SyntaxErrMesMethodDefaultArgumentNum2 = "関数{0}の引数の数が正しくありません";
        public const string SyntaxErrMesMethodDefaultArgumentNotNullable0 = "関数{0}の第{1}引数を省略できません";
        public const string SyntaxErrMesMethodDefaultArgumentType0 = "関数{0}の第{1}引数の型が正しくありません";
        public const string SyntaxErrMesMethodGraphicsColorMatrix0 = "関数{0}のカラーマトリクス指定が正しくありません";
        public const string RuntimeErrMesMethodDefaultArgumentOutOfRange0 = "関数{0}の引数が範囲外です";
        public const string RuntimeErrMesMethodGraphicsID0 = "グラフィックIDが正しくありません";
        public const string RuntimeErrMesMethodGraphicsID1 = "グラフィックIDが範囲外です";
        public const string RuntimeErrMesMethodGWidth0 = "画像幅が正しくありません";
        public const string RuntimeErrMesMethodGWidth1 = "画像幅が範囲外です";
        public const string RuntimeErrMesMethodGHeight0 = "画像高さが正しくありません";
        public const string RuntimeErrMesMethodGHeight1 = "画像高さが範囲外です";
        public const string RuntimeErrMesMethodGDIPLUSOnly = "この画像関数はheadlessでは利用できません";
        public const string RuntimeErrMesMethodGColorMatrix0 = "カラーマトリクスが正しくありません";
        public const string RuntimeErrMesMethodColorARGB0 = "ARGB値が正しくありません";
        public const string RuntimeErrMesMethodCIMGCreateOutOfRange0 = "CIMGCreateの引数が範囲外です";
    }
}

namespace UnityEngine
{
    public struct Color
    {
        public float r;
        public float g;
        public float b;
        public float a;

        public Color(float r, float g, float b, float a)
        {
            this.r = r;
            this.g = g;
            this.b = b;
            this.a = a;
        }
    }

    public sealed class Texture2D
    {
        public int width = 1;
        public int height = 1;

        public Color GetPixel(int x, int y)
        {
            return new Color(0, 0, 0, 0);
        }

        public void SetPixel(int x, int y, Color color)
        {
        }
    }

    public static class ImageConversion
    {
        public static byte[] EncodeToPNG(Texture2D texture)
        {
            return Array.Empty<byte>();
        }
    }

    public static class Screen
    {
        public static int width = 800;
        public static int height = 600;
    }

    public static class Debug
    {
        public static void Log(object value)
        {
            Console.Error.WriteLine(value);
        }
    }
}
