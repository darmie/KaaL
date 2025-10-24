#![no_std]
#![no_main]

use kaal_sdk::{
    component::Component,
    printf,
    syscall,
    message::{Channel, ChannelConfig as MsgChannelConfig},
    channel_setup::{establish_channel, ChannelRole},
};
use kaal_tui::{screen, cursor, style, draw, ui, Color};

// Declare as application component
kaal_sdk::component! {
    name: "todo_app",
    type: Application,
    version: "0.1.0",
    capabilities: [],
    impl: TodoApp
}

const MAX_TODOS: usize = 16;
const MAX_TODO_LEN: usize = 64;
const SCREEN_WIDTH: usize = 80;
const SCREEN_HEIGHT: usize = 24;

#[derive(Clone, Copy)]
struct Todo {
    text: [u8; MAX_TODO_LEN],
    len: usize,
    completed: bool,
}

impl Todo {
    const fn new() -> Self {
        Self {
            text: [0u8; MAX_TODO_LEN],
            len: 0,
            completed: false,
        }
    }

    fn set_text(&mut self, text: &str) {
        self.len = text.len().min(MAX_TODO_LEN);
        self.text[..self.len].copy_from_slice(&text.as_bytes()[..self.len]);
    }

    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.text[..self.len]).unwrap_or("")
    }

    fn toggle(&mut self) {
        self.completed = !self.completed;
    }
}

pub struct TodoApp {
    todos: [Todo; MAX_TODOS],
    count: usize,
    selected: usize,
    input_buffer: [u8; MAX_TODO_LEN],
    input_len: usize,
    mode: Mode,
    input_channel: Channel<u8>,
}

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    Normal,   // Navigate and toggle todos
    Insert,   // Adding new todo
}

impl Component for TodoApp {
    fn init() -> kaal_sdk::Result<Self> {
        // Clear screen and show title
        screen::clear();
        cursor::home();

        printf!("[todo] Todo App starting...\n");
        printf!("[todo] Setting up input channel...\n");

        // Establish IPC channel with UART driver for input
        // Retry until uart_driver is ready (it may not have started yet)
        let input_channel = loop {
            match establish_channel("kaal.uart.output", 4096, ChannelRole::Consumer) {
                Ok(config) => {
                    let msg_config = MsgChannelConfig {
                        shared_memory: config.buffer_addr,
                        receiver_notify: config.notification_cap as u64,
                        sender_notify: config.notification_cap as u64,
                    };

                    break unsafe { Channel::receiver(msg_config) };
                }
                Err(_) => {
                    // UART driver not ready yet, yield and retry
                    syscall::yield_now();
                }
            }
        };

        printf!("[todo] Input channel established\n");

        Ok(Self {
            todos: [Todo::new(); MAX_TODOS],
            count: 0,
            selected: 0,
            input_buffer: [0u8; MAX_TODO_LEN],
            input_len: 0,
            mode: Mode::Normal,
            input_channel,
        })
    }

    fn run(&mut self) -> ! {
        // Add some sample todos
        self.add_todo("Press 'a' to add a new todo");
        self.add_todo("Press Space to toggle completion");
        self.add_todo("Press 'd' to delete a todo");
        self.add_todo("Press 'j' and 'k' to navigate");

        printf!("[todo] Starting UI...\n");
        ui::init();
        self.draw();

        loop {
            // Wait for input
            match self.input_channel.receive() {
                Ok(byte) => {
                    self.handle_input(byte);
                    self.draw();
                }
                Err(_) => {
                    syscall::yield_now();
                }
            }
        }
    }
}

impl TodoApp {

    fn draw(&self) {
        // Clear and draw title
        screen::clear();
        cursor::home();

        // Title bar
        style::bold();
        style::fg(Color::BrightCyan);
        ui::title_bar("  KaaL Todo App - Simple Task Manager  ", SCREEN_WIDTH);
        style::reset();

        // Draw box for todo list
        draw::box_double(3, 5, SCREEN_WIDTH - 10, SCREEN_HEIGHT - 8);

        // Draw todos
        for i in 0..self.count {
            let row = 4 + i;
            let todo = &self.todos[i];

            cursor::goto(row, 7);

            // Highlight selected item
            if i == self.selected && self.mode == Mode::Normal {
                style::fg(Color::BrightYellow);
                printf!("> ");
            } else {
                printf!("  ");
            }

            // Draw checkbox
            if todo.completed {
                style::fg(Color::BrightGreen);
                printf!("[✓] ");
            } else {
                style::fg(Color::White);
                printf!("[ ] ");
            }

            // Draw text (strikethrough if completed)
            if todo.completed {
                style::fg(Color::BrightBlack);
            } else {
                style::fg(Color::White);
            }
            printf!("{}", todo.as_str());
            style::reset();
        }

        // Status bar based on mode
        let status_row = SCREEN_HEIGHT - 3;
        cursor::goto(status_row, 1);
        draw::hline(SCREEN_WIDTH, "─");

        let status_text_row = status_row + 1;
        match self.mode {
            Mode::Normal => {
                draw::text_at(status_text_row, 3, "Commands: ");
                style::fg(Color::BrightCyan);
                printf!("[j/k]");
                style::reset();
                printf!(" Navigate  ");
                style::fg(Color::BrightCyan);
                printf!("[Space]");
                style::reset();
                printf!(" Toggle  ");
                style::fg(Color::BrightCyan);
                printf!("[a]");
                style::reset();
                printf!(" Add  ");
                style::fg(Color::BrightCyan);
                printf!("[d]");
                style::reset();
                printf!(" Delete  ");
                style::fg(Color::BrightCyan);
                printf!("[q]");
                style::reset();
                printf!(" Quit");
            }
            Mode::Insert => {
                draw::text_at(status_text_row, 3, "Add new todo: ");
                style::fg(Color::BrightGreen);
                let input_str = core::str::from_utf8(&self.input_buffer[..self.input_len])
                    .unwrap_or("");
                printf!("{}", input_str);
                style::reset();
                printf!("█  ");
                draw::text_at(status_text_row + 1, 3, "(Press Enter to save, Esc to cancel)");
            }
        }

        cursor::hide();
    }

    fn add_todo(&mut self, text: &str) {
        if self.count < MAX_TODOS && !text.is_empty() {
            self.todos[self.count].set_text(text);
            self.count += 1;
        }
    }

    fn delete_selected(&mut self) {
        if self.count == 0 {
            return;
        }

        // Shift todos down
        for i in self.selected..self.count - 1 {
            self.todos[i] = self.todos[i + 1];
        }
        self.count -= 1;

        // Adjust selection
        if self.selected >= self.count && self.count > 0 {
            self.selected = self.count - 1;
        }
    }

    fn toggle_selected(&mut self) {
        if self.count > 0 {
            self.todos[self.selected].toggle();
        }
    }

    fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    fn move_down(&mut self) {
        if self.selected < self.count.saturating_sub(1) {
            self.selected += 1;
        }
    }

    fn handle_input(&mut self, ch: u8) {
        match self.mode {
            Mode::Normal => self.handle_normal_input(ch),
            Mode::Insert => self.handle_insert_input(ch),
        }
    }

    fn handle_normal_input(&mut self, ch: u8) {
        match ch {
            b'q' => {
                // Clean up TUI and exit by looping forever
                // In a real system, this would signal shutdown
                cursor::show();
                style::reset();
                screen::clear();
                cursor::home();
                printf!("\nTodo app exited. System halted.\n");
                loop {
                    syscall::yield_now();
                }
            }
            b'j' | b'J' => self.move_down(),
            b'k' | b'K' => self.move_up(),
            b' ' => self.toggle_selected(),
            b'a' | b'A' => {
                self.mode = Mode::Insert;
                self.input_len = 0;
            }
            b'd' | b'D' => self.delete_selected(),
            _ => {}
        }
    }

    fn handle_insert_input(&mut self, ch: u8) {
        match ch {
            b'\n' | b'\r' => {
                // Save todo - copy to avoid borrow issues
                if self.input_len > 0 {
                    let mut temp_buf = [0u8; MAX_TODO_LEN];
                    temp_buf[..self.input_len].copy_from_slice(&self.input_buffer[..self.input_len]);
                    let text = core::str::from_utf8(&temp_buf[..self.input_len]).unwrap_or("");
                    self.add_todo(text);
                }
                self.mode = Mode::Normal;
                self.input_len = 0;
            }
            0x1b => {
                // ESC - cancel
                self.mode = Mode::Normal;
                self.input_len = 0;
            }
            0x7f | 0x08 => {
                // Backspace
                if self.input_len > 0 {
                    self.input_len -= 1;
                }
            }
            _ if ch >= 0x20 && ch < 0x7f => {
                // Printable character
                if self.input_len < MAX_TODO_LEN {
                    self.input_buffer[self.input_len] = ch;
                    self.input_len += 1;
                }
            }
            _ => {}
        }
    }
}
