//! PL011 UART Driver
//!
//! Serial driver for ARM PL011 UART hardware. Supports interrupt-driven
//! receive with 4KB ring buffer and blocking transmit.
//!
//! Provides serial I/O to applications via shared memory IPC channel.
//!
//! # Testing
//! Enable the `notepad` component in components.toml to test the UART driver
//! with a real application. The notepad provides a simple text editor that
//! demonstrates UART input/output handling.

#![no_std]
#![no_main]

mod pl011;
mod ring_buffer;

use kaal_sdk::{
    component::Component,
    printf,
    syscall,
    message::{Channel, ChannelConfig as MsgChannelConfig},
    channel_setup::{establish_channel, ChannelRole},
};
use pl011::Pl011;
use ring_buffer::RingBuffer;

// Declare this as a driver component
kaal_sdk::component! {
    name: "uart_driver",
    type: Driver,
    version: "0.1.0",
    capabilities: ["caps:allocate", "irq:control", "memory:map"],
    impl: UartDriver
}

/// UART Driver
pub struct UartDriver {
    uart: Pl011,
    rx_buffer: RingBuffer<4096>,
    notification_cap: usize,
    irq_handler_slot: usize,
    irq_count: u32,
    char_count: u32,
    output_channel: Option<Channel<u8>>,
}

// Platform constants (from build-config.toml)
const UART0_BASE: usize = 0x09000000;  // UART0 MMIO base address
const UART0_SIZE: usize = 0x1000;      // 4KB MMIO region
const IRQ_CONTROL_SLOT: usize = 1;     // IRQControl capability from root-task (slot 0 is reserved)
const UART0_IRQ: usize = 33;           // UART0 IRQ number

/// IPC buffer size for output channel (4KB)
const IPC_BUFFER_SIZE: usize = 4096;

impl Component for UartDriver {
    fn init() -> kaal_sdk::Result<Self> {

        // Map UART MMIO region
        printf!("[uart_driver] Mapping UART0 MMIO: {:#x} ({} bytes)\n", UART0_BASE, UART0_SIZE);

        let uart_virt = match unsafe {
            syscall::memory_map(UART0_BASE, UART0_SIZE, 0x3) // RW permissions
        } {
            Ok(virt) => {
                printf!("  ✓ Mapped to virtual address: {:#x}\n", virt);
                virt
            }
            Err(_) => {
                printf!("  ✗ FAIL: Failed to map UART MMIO\n");
                printf!("  Driver requires memory:map capability\n");
                return Err(kaal_sdk::Error::SyscallFailed);
            }
        };

        // Initialize UART hardware
        let mut uart = unsafe { Pl011::new(uart_virt) };
        unsafe { uart.init(); }
        printf!("[uart_driver] Initialized: 115200 8N1, FIFOs enabled\n");

        // Create notification for UART IRQ
        let notification_cap = syscall::notification_create()?;
        let irq_handler_slot = syscall::cap_allocate()?;

        // Bind UART IRQ to notification
        printf!("[uart_driver] Binding IRQ {} to notification\n", UART0_IRQ);
        match unsafe {
            syscall::irq_handler_get(
                IRQ_CONTROL_SLOT,
                UART0_IRQ,
                notification_cap,
                irq_handler_slot,
            )
        } {
            Ok(()) => {
                printf!("[uart_driver] IRQ {} bound successfully\n", UART0_IRQ);
            }
            Err(_) => {
                printf!("[uart_driver] WARN: IRQ binding failed (requires IRQControl)\n");
            }
        }

        printf!("[uart_driver] Ready (MMIO: {:#x}, IRQ: {})\n", uart_virt, UART0_IRQ);
        uart.write_str("\r\nUART driver online\r\n");

        // Establish IPC channel with notepad for output
        printf!("[uart_driver] Establishing output channel to notepad...\n");
        let output_channel = match establish_channel("kaal.uart.output", IPC_BUFFER_SIZE, ChannelRole::Producer) {
            Ok(config) => {
                printf!("[uart_driver] Output channel established (buffer: {:#x})\n", config.buffer_addr);

                // Initialize the SharedRing structure in shared memory
                use kaal_sdk::ipc::SharedRing;
                use core::ptr;

                let ring_ptr = config.buffer_addr as *mut SharedRing<u8, 256>;
                unsafe {
                    ptr::write(ring_ptr, SharedRing::new());
                    printf!("[uart_driver] Initialized SharedRing<u8, 256> in shared memory\n");
                }

                let msg_config = MsgChannelConfig {
                    shared_memory: config.buffer_addr,
                    receiver_notify: config.notification_cap as u64,
                    sender_notify: config.notification_cap as u64,
                };
                Some(unsafe { Channel::sender(msg_config) })
            }
            Err(e) => {
                printf!("[uart_driver] WARN: Failed to establish output channel: {}\n", e);
                printf!("[uart_driver] Will buffer input but not forward to applications\n");
                None
            }
        };

        Ok(Self {
            uart,
            rx_buffer: RingBuffer::new(),
            notification_cap,
            irq_handler_slot,
            irq_count: 0,
            char_count: 0,
            output_channel,
        })
    }

    fn run(&mut self) -> ! {
        loop {
            // Wait for notification (blocks until IRQ fires)
            match syscall::wait(self.notification_cap) {
                Ok(_badge) => {
                    self.irq_count += 1;

                    // Handle RX interrupt
                    if self.uart.has_rx_interrupt() {
                        self.handle_rx_interrupt();
                        self.uart.clear_rx_interrupts();
                    }

                    // Acknowledge IRQ to re-enable it
                    if let Err(_) = unsafe { syscall::irq_handler_ack(self.irq_handler_slot) } {
                        printf!("[uart_driver] ERROR: Failed to ACK IRQ\n");
                    }
                }
                Err(_) => {
                    // Wait failed - yield and retry
                    syscall::yield_now();
                }
            }
        }
    }
}

impl UartDriver {
    /// Handle receive interrupt - buffer incoming data
    fn handle_rx_interrupt(&mut self) {
        // Read all available bytes from UART FIFO
        while let Some(byte) = self.uart.read_byte() {
            self.char_count += 1;

            // Echo character back to UART
            self.uart.write_byte(byte);

            // Send to notepad via IPC channel if established
            if let Some(ref mut channel) = self.output_channel {
                if let Err(_) = channel.send(byte) {
                    printf!("[uart_driver] WARN: Failed to send char to notepad\n");
                }
            } else {
                // No channel - store in buffer
                if self.rx_buffer.push(byte).is_err() {
                    printf!("[uart_driver] WARN: RX buffer overflow!\n");
                }
            }
        }
    }

    /// Write data to UART (for applications to use via IPC)
    #[allow(dead_code)]
    pub fn write(&mut self, data: &[u8]) {
        for &byte in data {
            self.uart.write_byte(byte);
        }
    }

    /// Read buffered data (for applications to use via IPC)
    #[allow(dead_code)]
    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        let mut count = 0;
        for i in 0..buf.len() {
            if let Some(byte) = self.rx_buffer.pop() {
                buf[i] = byte;
                count += 1;
            } else {
                break;
            }
        }
        count
    }
}
