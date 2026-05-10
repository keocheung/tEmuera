use std::io::{self, Write};

use crate::error::{HeadlessError, Result};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Rgb {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Rgb {
    pub const BLACK: Self = Self::new(0, 0, 0);
    pub const EMUERA_ORANGE: Self = Self::new(255, 128, 64);

    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct TextStyle {
    pub foreground: Option<Rgb>,
    pub background: Option<Rgb>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikeout: bool,
}

pub struct Terminal<W> {
    writer: W,
}

impl Terminal<io::Stdout> {
    pub fn stdout() -> Self {
        Self {
            writer: io::stdout(),
        }
    }
}

impl<W: Write> Terminal<W> {
    pub fn apply_palette(&mut self, foreground: Rgb, background: Rgb) -> Result<()> {
        write!(
            self.writer,
            "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m",
            foreground.red,
            foreground.green,
            foreground.blue,
            background.red,
            background.green,
            background.blue
        )
        .map_err(|err| HeadlessError::io("write terminal palette", err))
    }

    pub fn clear(&mut self) -> Result<()> {
        write!(self.writer, "\x1b[2J\x1b[H").map_err(|err| HeadlessError::io("clear terminal", err))
    }

    pub fn reset(&mut self) -> Result<()> {
        write!(self.writer, "\x1b[0m").map_err(|err| HeadlessError::io("reset terminal", err))
    }

    pub fn writeln_styled(&mut self, text: &str, style: TextStyle) -> Result<()> {
        self.write_style(style)?;
        self.writer
            .write_all(text.as_bytes())
            .map_err(|err| HeadlessError::io("write terminal text", err))?;
        self.writer
            .write_all(b"\x1b[0m\n")
            .map_err(|err| HeadlessError::io("write terminal reset", err))
    }

    pub fn flush(&mut self) -> Result<()> {
        self.writer
            .flush()
            .map_err(|err| HeadlessError::io("flush terminal", err))
    }

    fn write_style(&mut self, style: TextStyle) -> Result<()> {
        if let Some(color) = style.foreground {
            write!(
                self.writer,
                "\x1b[38;2;{};{};{}m",
                color.red, color.green, color.blue
            )
            .map_err(|err| HeadlessError::io("write foreground color", err))?;
        }
        if let Some(color) = style.background {
            write!(
                self.writer,
                "\x1b[48;2;{};{};{}m",
                color.red, color.green, color.blue
            )
            .map_err(|err| HeadlessError::io("write background color", err))?;
        }
        if style.bold {
            self.writer
                .write_all(b"\x1b[1m")
                .map_err(|err| HeadlessError::io("write bold style", err))?;
        }
        if style.italic {
            self.writer
                .write_all(b"\x1b[3m")
                .map_err(|err| HeadlessError::io("write italic style", err))?;
        }
        if style.underline {
            self.writer
                .write_all(b"\x1b[4m")
                .map_err(|err| HeadlessError::io("write underline style", err))?;
        }
        if style.strikeout {
            self.writer
                .write_all(b"\x1b[9m")
                .map_err(|err| HeadlessError::io("write strikeout style", err))?;
        }
        Ok(())
    }
}

pub fn emuera_columns(text: &str) -> usize {
    text.chars()
        .map(|ch| if is_emuera_half_width(ch) { 1 } else { 2 })
        .sum()
}

fn is_emuera_half_width(ch: char) -> bool {
    matches!(
        ch,
        '\u{0000}'..='\u{00ff}' | '\u{ff61}'..='\u{ff9f}' | '\u{ffe8}'..='\u{ffee}'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emuera_width_matches_basic_ascii_and_cjk_expectations() {
        assert_eq!(emuera_columns("abc"), 3);
        assert_eq!(emuera_columns("紅魔館"), 6);
        assert_eq!(emuera_columns("■┃＜＞"), 8);
    }
}
