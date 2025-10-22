//! Simple ring buffer for UART received data

/// Ring buffer for bytes
pub struct RingBuffer<const N: usize> {
    buffer: [u8; N],
    head: usize,  // Write position
    tail: usize,  // Read position
    count: usize, // Number of items
}

impl<const N: usize> RingBuffer<N> {
    /// Create a new empty ring buffer
    pub const fn new() -> Self {
        Self {
            buffer: [0; N],
            head: 0,
            tail: 0,
            count: 0,
        }
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Check if the buffer is full
    pub fn is_full(&self) -> bool {
        self.count == N
    }

    /// Get the number of bytes in the buffer
    pub fn len(&self) -> usize {
        self.count
    }

    /// Push a byte into the buffer
    ///
    /// Returns `Ok(())` on success, `Err(())` if buffer is full
    pub fn push(&mut self, byte: u8) -> Result<(), ()> {
        if self.is_full() {
            return Err(());
        }

        self.buffer[self.head] = byte;
        self.head = (self.head + 1) % N;
        self.count += 1;
        Ok(())
    }

    /// Pop a byte from the buffer
    ///
    /// Returns `Some(byte)` if available, `None` if empty
    pub fn pop(&mut self) -> Option<u8> {
        if self.is_empty() {
            return None;
        }

        let byte = self.buffer[self.tail];
        self.tail = (self.tail + 1) % N;
        self.count -= 1;
        Some(byte)
    }

    /// Peek at the next byte without removing it
    pub fn peek(&self) -> Option<u8> {
        if self.is_empty() {
            None
        } else {
            Some(self.buffer[self.tail])
        }
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
        self.count = 0;
    }
}
