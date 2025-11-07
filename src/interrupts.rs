// Interrupt handler stubs
#[no_mangle]
pub extern "C" fn DefaultHandler() {
    loop {}
}

// Stack guard symbol (if stack protection is enabled)
// Note: __stack_chk_fail is provided by esp-hal, so we only need the guard
#[no_mangle]
pub static mut __stack_chk_guard: u32 = 0;

