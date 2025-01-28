#![no_std]
#![no_main]

mod writer;
use writer::FrameBufferWriter;
use bootloader_api::config::Mapping;
use x86_64::instructions::hlt;

pub static BOOTLOADER_CONFIG: bootloader_api::BootloaderConfig = {
    let mut config = bootloader_api::BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config.kernel_stack_size = 100 * 1024; // 100 KiB
    config
};

bootloader_api::entry_point!(my_entry_point, config = &BOOTLOADER_CONFIG);

fn my_entry_point(_boot_info: &'static mut bootloader_api::BootInfo) -> ! {
    let frame_buffer_info = _boot_info.framebuffer.as_mut().unwrap().info();
    let buffer = _boot_info.framebuffer.as_mut().unwrap().buffer_mut();
    let mut frame_buffer_writer = FrameBufferWriter::new(buffer, frame_buffer_info);

    frame_buffer_writer.print("Hello, world!\nThis is a test.\n\\cBlue Text\tIndented Text");

    loop {
        hlt(); // Stop x86_64 from being unnecessarily busy while looping
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        hlt(); // Halt in case of panic
    }
}