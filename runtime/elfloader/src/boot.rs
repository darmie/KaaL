// Boot sequence management

use crate::uart_println;
use crate::{BootInfo, payload::Payload};

/// Embedded payload (added by elfloader-builder)
/// This will be patched in by the build tool
#[link_section = ".rodata.payload"]
#[no_mangle]
static PAYLOAD_DATA: [u8; 4] = [0, 0, 0, 0]; // Placeholder, will be replaced

/// Pointer to actual payload (set by builder)
#[link_section = ".data.payload_ptr"]
#[no_mangle]
static mut PAYLOAD_START: usize = 0;

#[link_section = ".data.payload_size"]
#[no_mangle]
static mut PAYLOAD_SIZE: usize = 0;

pub fn load_images() -> BootInfo {
    uart_println!("Deserializing payload...");

    // Get payload data
    let payload_slice = unsafe {
        if PAYLOAD_START == 0 || PAYLOAD_SIZE == 0 {
            panic!("Payload not initialized! Run kaal-elfloader-builder first.");
        }
        core::slice::from_raw_parts(PAYLOAD_START as *const u8, PAYLOAD_SIZE)
    };

    uart_println!("Payload size: {} bytes", payload_slice.len());

    // Deserialize payload metadata
    let (payload, data): (Payload, &[u8]) = postcard::take_from_bytes(payload_slice)
        .expect("Failed to deserialize payload");

    uart_println!("Payload metadata:");
    uart_println!("  Kernel entry: {:#x}", payload.kernel_entry);
    uart_println!("  User entry:   {:#x}", payload.user_entry);
    uart_println!("  Kernel regions: {}", payload.kernel_regions.len());
    uart_println!("  User regions:   {}", payload.user_regions.len());

    // Load regions into physical memory
    unsafe {
        payload.load_to_memory(data);
    }

    let (user_start, user_end) = payload.user_paddr_range();

    uart_println!("Images loaded successfully!");

    BootInfo {
        user_img_start: user_start,
        user_img_end: user_end,
        pv_offset: 0, // Will be calculated based on kernel mapping
        user_entry: payload.user_entry,
        dtb_addr: 0, // Will be set from dtb parameter
        dtb_size: 0,
    }
}
