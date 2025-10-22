//! Notepad - Terminal Text Editor
//!
//! Line-based text editor with UART integration.
//!
//! # Commands
//! - Type: Add text to current line
//! - Enter: Save line and start new
//! - Backspace: Delete last character
//! - Ctrl+L: List all saved lines
//! - Ctrl+C: Clear all lines
//! - Ctrl+Q: Quit and shutdown system

#![no_std]
#![no_main]

use kaal_sdk::{
    component::Component,
    printf,
    syscall,
};

// Declare as application component
kaal_sdk::component! {
    name: "notepad",
    type: Application,
    version: "0.1.0",
    capabilities: [],
    impl: Notepad
}

/// Text editor state
pub struct Notepad {
    lines: [Line; 32],          // Maximum 32 lines
    line_count: usize,
    current_line: Line,
    current_pos: usize,
    char_count: usize,
}

/// A single line of text
struct Line {
    data: [u8; 128],  // Maximum 128 characters per line
    len: usize,
}

impl Line {
    const fn new() -> Self {
        Self {
            data: [0; 128],
            len: 0,
        }
    }

    fn push(&mut self, ch: u8) -> bool {
        if self.len >= 128 {
            return false;
        }
        self.data[self.len] = ch;
        self.len += 1;
        true
    }

    fn pop(&mut self) -> Option<u8> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        Some(self.data[self.len])
    }

    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.data[..self.len]).unwrap_or("<invalid utf8>")
    }

    fn clear(&mut self) {
        self.len = 0;
    }
}

impl Component for Notepad {
    fn init() -> kaal_sdk::Result<Self> {
        printf!("\n");
        printf!("╔═══════════════════════════════════════╗\n");
        printf!("║        KaaL Notepad v0.1.0            ║\n");
        printf!("╚═══════════════════════════════════════╝\n");
        printf!("\n");
        printf!("Simple line-based text editor\n");
        printf!("\n");
        printf!("Commands:\n");
        printf!("  Type        - Add text to current line\n");
        printf!("  Enter       - Save line and start new one\n");
        printf!("  Backspace   - Delete last character\n");
        printf!("  Ctrl+L (^L) - List all saved lines\n");
        printf!("  Ctrl+C (^C) - Clear all lines\n");
        printf!("  Ctrl+Q (^Q) - Quit and shutdown\n");
        printf!("\n");
        printf!("Ready. Start typing!\n");
        printf!("> ");

        Ok(Self {
            lines: [Line::new(); 32],
            line_count: 0,
            current_line: Line::new(),
            current_pos: 0,
            char_count: 0,
        })
    }

    fn run(&mut self) -> ! {
        // TODO: Set up IPC channel with UART driver
        // For now, demonstrate the component structure

        loop {
            // TODO: Wait for UART characters via IPC channel
            syscall::yield_now();
        }
    }
}

impl Notepad {
    /// Process a single character of input
    #[allow(dead_code)]
    fn process_char(&mut self, ch: u8) {
        match ch {
            // Newline/Enter - save current line
            b'\n' | b'\r' => {
                if self.line_count < 32 {
                    // Copy current line to saved lines
                    self.lines[self.line_count] = core::mem::replace(
                        &mut self.current_line,
                        Line::new()
                    );
                    self.line_count += 1;
                    self.current_pos = 0;

                    printf!("\n[Line {} saved]\n> ", self.line_count);
                } else {
                    printf!("\n[ERROR: Maximum 32 lines reached]\n> ");
                    self.current_line.clear();
                    self.current_pos = 0;
                }
            }

            // Backspace - delete last character
            0x7F | 0x08 => {
                if self.current_line.pop().is_some() {
                    self.current_pos -= 1;
                    // Send backspace sequence to terminal
                    printf!("\x08 \x08");
                }
            }

            // Ctrl+L - list all lines
            0x0C => {
                printf!("\n\n=== Saved Lines ({}) ===\n", self.line_count);
                for i in 0..self.line_count {
                    printf!("{:2}: {}\n", i + 1, self.lines[i].as_str());
                }
                printf!("========================\n> ");
                // Redisplay current line
                for i in 0..self.current_line.len {
                    printf!("{}", self.current_line.data[i] as char);
                }
            }

            // Ctrl+C - clear all lines
            0x03 => {
                self.line_count = 0;
                self.current_line.clear();
                self.current_pos = 0;
                printf!("\n[All lines cleared]\n> ");
            }

            // Ctrl+Q - quit and shutdown
            0x11 => {
                printf!("\n\n");
                printf!("=== Notepad Session Summary ===\n");
                printf!("Lines saved:  {}\n", self.line_count);
                printf!("Total chars:  {}\n", self.char_count);
                printf!("==============================\n");
                printf!("\nShutting down...\n");
                syscall::shutdown();
            }

            // Printable character - add to current line
            0x20..=0x7E => {
                if self.current_line.push(ch) {
                    self.current_pos += 1;
                    self.char_count += 1;
                    // Echo character (UART driver will handle actual echo)
                    // printf!("{}", ch as char);
                } else {
                    printf!("\n[Line full - press Enter to save]\n");
                }
            }

            // Ignore other control characters
            _ => {}
        }
    }

    /// Display statistics
    #[allow(dead_code)]
    fn show_stats(&self) {
        printf!("\n");
        printf!("=== Notepad Statistics ===\n");
        printf!("Lines saved:    {}\n", self.line_count);
        printf!("Current line:   {} chars\n", self.current_line.len);
        printf!("Total chars:    {}\n", self.char_count);
        printf!("=========================\n");
    }
}
