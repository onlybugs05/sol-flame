#[no_mangle]
pub extern "C" fn entrypoint(input: *const u8) -> u64 {
    process_instruction(input)
}

#[inline(never)]
pub fn process_instruction(input: *const u8) -> u64 {
    borsh_deser(input) + 42
}

#[inline(never)]
pub fn borsh_deser(_input: *const u8) -> u64 {
    100
}
