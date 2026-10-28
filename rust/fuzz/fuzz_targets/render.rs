#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| tok_fuzz::oracles::render(data));
