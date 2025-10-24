#![no_std]
#![no_main]

use kaal_sdk::{
    component::Component,
    printf,
    syscall,
    message::Channel,
    channel_setup::{establish_channel, ChannelRole, ChannelConfig},
    message::ChannelConfig as MsgChannelConfig,
};
use kaal_tui::{screen, cursor, style, draw, ui, Color};

// Declare as application component
kaal_sdk::component! {
    name: "system_monitor",
    type: Application,
    version: "0.1.0",
    capabilities: [],
    impl: SystemMonitor
}

const SCREEN_WIDTH: usize = 80;

pub struct SystemMonitor {
    input_channel: Channel<u8>,
    refresh_counter: usize,
}

impl Component for SystemMonitor {
    fn init() -> kaal_sdk::Result<Self> {
        // Establish IPC channel with UART driver for input
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
                    syscall::yield_now();
                }
            }
        };

        Ok(Self {
            input_channel,
            refresh_counter: 0,
        })
    }

    fn run(&mut self) -> ! {
        ui::init();
        self.draw_full_ui();

        loop {
            // Wait for input
            match self.input_channel.receive() {
                Ok(byte) => {
                    self.handle_input(byte);
                }
                Err(_) => {
                    syscall::yield_now();
                }
            }
        }
    }
}

impl SystemMonitor {
    fn draw_full_ui(&self) {
        screen::clear();
        cursor::home();

        // Draw banner
        self.draw_banner();

        // Draw system status section
        self.draw_system_status();

        // Draw process table header
        self.draw_process_section();

        // Draw demo applications section
        self.draw_demo_section();

        // Draw command bar
        self.draw_command_bar();

        cursor::hide();
    }

    fn draw_banner(&self) {
        cursor::goto(2, 1);
        style::fg(Color::BrightCyan);
        style::bold();

        // KaaL ASCII art (centered)
        let banner_lines = [
            "     $$\\   $$\\                    $$\\                      ",
            "     $$ | $$  |                   $$ |                     ",
            "     $$ |$$  / $$$$$$\\   $$$$$$\\  $$ |                     ",
            "     $$$$$  /  \\____$$\\  \\____$$\\ $$ |                     ",
            "     $$  $$<   $$$$$$$ | $$$$$$$ |$$ |                     ",
            "     $$ |\\$$\\ $$  __$$ |$$  __$$ |$$ |                     ",
            "     $$ | \\$$\\\\$$$$$$$ |\\$$$$$$$ |$$$$$$$$\\                ",
            "     \\__|  \\__|\\______| \\_______|\\________|               ",
        ];

        for (i, line) in banner_lines.iter().enumerate() {
            cursor::goto(2 + i, 10);
            printf!("{}", line);
        }

        cursor::goto(11, 18);
        style::reset();
        style::fg(Color::BrightWhite);
        printf!("KaaL System Monitor v0.1.0");

        cursor::goto(12, 18);
        style::fg(Color::White);
        printf!("Capability-Based Microkernel");

        style::reset();
    }

    fn draw_system_status(&self) {
        cursor::goto(14, 1);
        draw::hline(SCREEN_WIDTH, "─");

        cursor::goto(15, 2);
        style::fg(Color::BrightYellow);
        style::bold();
        printf!("SYSTEM STATUS");
        style::reset();

        cursor::goto(16, 1);
        draw::hline(SCREEN_WIDTH, "─");

        // Memory info (we'll add syscalls for this later)
        cursor::goto(17, 2);
        style::fg(Color::White);
        printf!("Memory:  ");
        style::fg(Color::BrightGreen);
        printf!("[");
        style::fg(Color::Green);
        for _ in 0..16 {
            printf!("█");
        }
        style::fg(Color::BrightBlack);
        for _ in 0..24 {
            printf!("░");
        }
        style::fg(Color::BrightGreen);
        printf!("]");
        style::fg(Color::White);
        printf!(" 5.2 MB / 128 MB (4%)");
        style::reset();

        cursor::goto(18, 2);
        printf!("Frames:  31684 free / 32768 total");

        cursor::goto(19, 2);
        printf!("Uptime:  0d 0h {}m {}s", self.refresh_counter / 60, self.refresh_counter % 60);
    }

    fn draw_process_section(&self) {
        cursor::goto(20, 1);
        draw::hline(SCREEN_WIDTH, "─");

        cursor::goto(21, 2);
        style::fg(Color::BrightYellow);
        style::bold();
        printf!("PROCESSES (2)");
        style::reset();
        style::fg(Color::BrightBlack);
        printf!("                                 [Press 'k' + number to kill]");
        style::reset();

        cursor::goto(22, 1);
        draw::hline(SCREEN_WIDTH, "─");

        // Table header
        cursor::goto(23, 2);
        style::fg(Color::BrightCyan);
        printf!("PID        Name              Priority    State        Memory");
        style::reset();

        cursor::goto(24, 2);
        style::fg(Color::BrightBlack);
        printf!("────────── ───────────────── ─────────── ──────────── ──────────");
        style::reset();

        // Process list (hardcoded for now, will add syscalls later)
        let processes = [
            ("1083781120", "uart_driver", "50", "Running", "32 KB"),
            ("1083846656", "system_monitor", "100", "Running", "16 KB"),
        ];

        for (i, (pid, name, priority, state, memory)) in processes.iter().enumerate() {
            cursor::goto(25 + i, 2);
            style::fg(Color::White);
            printf!("{:<10} ", pid);
            style::fg(Color::BrightWhite);
            printf!("{:<17} ", name);
            style::fg(Color::Yellow);
            printf!("{:<11} ", priority);
            style::fg(Color::BrightGreen);
            printf!("{:<12} ", state);
            style::fg(Color::Cyan);
            printf!("{}", memory);
            style::reset();
        }
    }

    fn draw_demo_section(&self) {
        cursor::goto(28, 1);
        draw::hline(SCREEN_WIDTH, "─");

        cursor::goto(29, 2);
        style::fg(Color::BrightYellow);
        style::bold();
        printf!("DEMO APPLICATIONS");
        style::reset();

        let demos = [
            ("1", "Notepad", "Line-based text editor"),
            ("2", "Todo App", "Vi-style task manager"),
            ("3", "Hex Editor", "Binary file viewer (coming soon)"),
        ];

        for (i, (key, name, desc)) in demos.iter().enumerate() {
            cursor::goto(30 + i, 2);
            style::fg(Color::BrightCyan);
            printf!("[{}]", key);
            style::fg(Color::BrightWhite);
            printf!(" {:<11}", name);
            style::fg(Color::White);
            printf!(" - {}", desc);
            style::reset();
        }
    }

    fn draw_command_bar(&self) {
        cursor::goto(34, 1);
        draw::hline(SCREEN_WIDTH, "─");

        cursor::goto(35, 2);
        style::fg(Color::BrightGreen);
        printf!("[r]");
        style::fg(Color::White);
        printf!(" Refresh  ");

        style::fg(Color::BrightCyan);
        printf!("[1-9]");
        style::fg(Color::White);
        printf!(" Launch  ");

        style::fg(Color::BrightRed);
        printf!("[k]");
        style::fg(Color::White);
        printf!(" Kill Process  ");

        style::fg(Color::BrightMagenta);
        printf!("[q]");
        style::fg(Color::White);
        printf!(" Quit");

        style::reset();
    }

    fn draw_status_message(&self, message: &str, is_error: bool) {
        cursor::goto(36, 2);
        screen::clear_line();

        if is_error {
            style::fg(Color::BrightRed);
            printf!("✗ ");
        } else {
            style::fg(Color::BrightGreen);
            printf!("✓ ");
        }

        style::fg(Color::White);
        printf!("{}", message);
        style::reset();
    }

    fn handle_input(&mut self, ch: u8) {
        match ch {
            b'q' | b'Q' => {
                // Quit - clean up and halt
                cursor::show();
                style::reset();
                screen::clear();
                cursor::home();
                printf!("\nSystem monitor exited.\n");
                loop {
                    syscall::yield_now();
                }
            }
            b'r' | b'R' => {
                // Refresh
                self.refresh_counter += 1;
                self.draw_full_ui();
                self.draw_status_message("Display refreshed", false);
            }
            b'1' => {
                self.draw_status_message("Launching Notepad... (spawning not yet implemented)", false);
            }
            b'2' => {
                self.draw_status_message("Launching Todo App... (spawning not yet implemented)", false);
            }
            b'3' => {
                self.draw_status_message("Hex Editor coming soon!", false);
            }
            b'k' | b'K' => {
                self.draw_status_message("Process killing not yet implemented", false);
            }
            _ => {
                // Ignore other keys
            }
        }
    }
}
