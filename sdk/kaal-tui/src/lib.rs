//! KaaL TUI Library - Terminal User Interface primitives
//!
//! Provides ANSI escape sequence utilities for building text-based UIs:
//! - Screen clearing and cursor control
//! - Colors and text attributes
//! - Box drawing characters
//! - Simple layout helpers

#![no_std]

use kaal_sdk::printf;

/// ANSI Color codes
#[derive(Copy, Clone, Debug)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Red = 1,
    Green = 2,
    Yellow = 3,
    Blue = 4,
    Magenta = 5,
    Cyan = 6,
    White = 7,
    BrightBlack = 8,
    BrightRed = 9,
    BrightGreen = 10,
    BrightYellow = 11,
    BrightBlue = 12,
    BrightMagenta = 13,
    BrightCyan = 14,
    BrightWhite = 15,
}

/// Text attributes
#[derive(Copy, Clone, Debug)]
pub enum Attribute {
    Reset = 0,
    Bold = 1,
    Dim = 2,
    Italic = 3,
    Underline = 4,
    Blink = 5,
    Reverse = 7,
    Hidden = 8,
}

/// Screen control functions
pub mod screen {
    use super::*;

    /// Clear the entire screen
    pub fn clear() {
        printf!("\x1b[2J");
    }

    /// Clear from cursor to end of screen
    pub fn clear_to_end() {
        printf!("\x1b[0J");
    }

    /// Clear from cursor to start of screen
    pub fn clear_to_start() {
        printf!("\x1b[1J");
    }

    /// Clear the current line
    pub fn clear_line() {
        printf!("\x1b[2K");
    }

    /// Save current screen and switch to alternate buffer (like vim/less)
    pub fn enter_alternate() {
        printf!("\x1b[?1049h");
    }

    /// Restore original screen from alternate buffer
    pub fn exit_alternate() {
        printf!("\x1b[?1049l");
    }

    /// Reset terminal to initial state
    pub fn reset() {
        printf!("\x1bc");
    }
}

/// Cursor control functions
pub mod cursor {
    use super::*;

    /// Move cursor to home position (1,1)
    pub fn home() {
        printf!("\x1b[H");
    }

    /// Move cursor to specific position (row, col) - 1-indexed
    pub fn goto(row: usize, col: usize) {
        printf!("\x1b[{};{}H", row, col);
    }

    /// Move cursor up by n lines
    pub fn up(n: usize) {
        printf!("\x1b[{}A", n);
    }

    /// Move cursor down by n lines
    pub fn down(n: usize) {
        printf!("\x1b[{}B", n);
    }

    /// Move cursor right by n columns
    pub fn right(n: usize) {
        printf!("\x1b[{}C", n);
    }

    /// Move cursor left by n columns
    pub fn left(n: usize) {
        printf!("\x1b[{}D", n);
    }

    /// Save cursor position
    pub fn save() {
        printf!("\x1b[s");
    }

    /// Restore cursor position
    pub fn restore() {
        printf!("\x1b[u");
    }

    /// Hide cursor
    pub fn hide() {
        printf!("\x1b[?25l");
    }

    /// Show cursor
    pub fn show() {
        printf!("\x1b[?25h");
    }
}

/// Color and style functions
pub mod style {
    use super::*;

    /// Set foreground color
    pub fn fg(color: Color) {
        if (color as u8) < 8 {
            printf!("\x1b[{}m", 30 + (color as u8));
        } else {
            printf!("\x1b[{}m", 82 + (color as u8));
        }
    }

    /// Set background color
    pub fn bg(color: Color) {
        if (color as u8) < 8 {
            printf!("\x1b[{}m", 40 + (color as u8));
        } else {
            printf!("\x1b[{}m", 92 + (color as u8));
        }
    }

    /// Set text attribute
    pub fn attr(attribute: Attribute) {
        printf!("\x1b[{}m", attribute as u8);
    }

    /// Reset all styles to default
    pub fn reset() {
        printf!("\x1b[0m");
    }

    /// Set bold text
    pub fn bold() {
        attr(Attribute::Bold);
    }

    /// Set underline text
    pub fn underline() {
        attr(Attribute::Underline);
    }

    /// Set reverse video (swap fg/bg)
    pub fn reverse() {
        attr(Attribute::Reverse);
    }
}

/// Box drawing characters (Unicode)
pub mod box_chars {
    // Single-line box drawing
    pub const HORIZONTAL: &str = "─";
    pub const VERTICAL: &str = "│";
    pub const TOP_LEFT: &str = "┌";
    pub const TOP_RIGHT: &str = "┐";
    pub const BOTTOM_LEFT: &str = "└";
    pub const BOTTOM_RIGHT: &str = "┘";
    pub const T_DOWN: &str = "┬";
    pub const T_UP: &str = "┴";
    pub const T_RIGHT: &str = "├";
    pub const T_LEFT: &str = "┤";
    pub const CROSS: &str = "┼";

    // Double-line box drawing
    pub const DOUBLE_HORIZONTAL: &str = "═";
    pub const DOUBLE_VERTICAL: &str = "║";
    pub const DOUBLE_TOP_LEFT: &str = "╔";
    pub const DOUBLE_TOP_RIGHT: &str = "╗";
    pub const DOUBLE_BOTTOM_LEFT: &str = "╚";
    pub const DOUBLE_BOTTOM_RIGHT: &str = "╝";

    // Block elements
    pub const BLOCK_FULL: &str = "█";
    pub const BLOCK_LIGHT: &str = "░";
    pub const BLOCK_MEDIUM: &str = "▒";
    pub const BLOCK_DARK: &str = "▓";
}

/// Drawing helper functions
pub mod draw {
    use super::*;

    /// Draw a horizontal line at current cursor position
    pub fn hline(width: usize, ch: &str) {
        for _ in 0..width {
            printf!("{}", ch);
        }
    }

    /// Draw a vertical line starting at current position
    pub fn vline(height: usize, ch: &str, start_row: usize, col: usize) {
        for i in 0..height {
            cursor::goto(start_row + i, col);
            printf!("{}", ch);
        }
    }

    /// Draw a box with single-line characters
    pub fn box_single(row: usize, col: usize, width: usize, height: usize) {
        // Top border
        cursor::goto(row, col);
        printf!("{}", box_chars::TOP_LEFT);
        hline(width - 2, box_chars::HORIZONTAL);
        printf!("{}", box_chars::TOP_RIGHT);

        // Sides
        for i in 1..height - 1 {
            cursor::goto(row + i, col);
            printf!("{}", box_chars::VERTICAL);
            cursor::goto(row + i, col + width - 1);
            printf!("{}", box_chars::VERTICAL);
        }

        // Bottom border
        cursor::goto(row + height - 1, col);
        printf!("{}", box_chars::BOTTOM_LEFT);
        hline(width - 2, box_chars::HORIZONTAL);
        printf!("{}", box_chars::BOTTOM_RIGHT);
    }

    /// Draw a box with double-line characters
    pub fn box_double(row: usize, col: usize, width: usize, height: usize) {
        // Top border
        cursor::goto(row, col);
        printf!("{}", box_chars::DOUBLE_TOP_LEFT);
        hline(width - 2, box_chars::DOUBLE_HORIZONTAL);
        printf!("{}", box_chars::DOUBLE_TOP_RIGHT);

        // Sides
        for i in 1..height - 1 {
            cursor::goto(row + i, col);
            printf!("{}", box_chars::DOUBLE_VERTICAL);
            cursor::goto(row + i, col + width - 1);
            printf!("{}", box_chars::DOUBLE_VERTICAL);
        }

        // Bottom border
        cursor::goto(row + height - 1, col);
        printf!("{}", box_chars::DOUBLE_BOTTOM_LEFT);
        hline(width - 2, box_chars::DOUBLE_HORIZONTAL);
        printf!("{}", box_chars::DOUBLE_BOTTOM_RIGHT);
    }

    /// Draw centered text at a specific row
    pub fn text_centered(row: usize, text: &str, screen_width: usize) {
        let text_len = text.len();
        if text_len < screen_width {
            let col = (screen_width - text_len) / 2;
            cursor::goto(row, col);
            printf!("{}", text);
        }
    }

    /// Draw text at specific position
    pub fn text_at(row: usize, col: usize, text: &str) {
        cursor::goto(row, col);
        printf!("{}", text);
    }
}

/// High-level UI helpers
pub mod ui {
    use super::*;

    /// Initialize a full-screen TUI application
    pub fn init() {
        screen::clear();
        cursor::home();
        cursor::hide();
    }

    /// Cleanup and exit TUI application
    pub fn cleanup() {
        cursor::show();
        style::reset();
        screen::clear();
        cursor::home();
    }

    /// Draw a title bar across the top of the screen
    pub fn title_bar(text: &str, width: usize) {
        cursor::goto(1, 1);
        style::reverse();
        printf!("{}", text);
        // Pad with spaces to fill width
        for _ in text.len()..width {
            printf!(" ");
        }
        style::reset();
    }

    /// Draw a status bar at the bottom of the screen
    pub fn status_bar(text: &str, row: usize, width: usize) {
        cursor::goto(row, 1);
        style::reverse();
        printf!("{}", text);
        // Pad with spaces to fill width
        for _ in text.len()..width {
            printf!(" ");
        }
        style::reset();
    }

    /// Draw a menu item (selectable)
    pub fn menu_item(row: usize, col: usize, text: &str, selected: bool) {
        cursor::goto(row, col);
        if selected {
            style::reverse();
            printf!("> {}", text);
            style::reset();
        } else {
            printf!("  {}", text);
        }
    }
}
