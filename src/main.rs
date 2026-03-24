#![no_std]
#![no_main]

mod lang_items;

use core::panic::PanicInfo;

#[unsafe(no_mangle)]
extern "C" fn start() {
    loop{};
}
