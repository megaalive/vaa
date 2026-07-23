#![no_main]

use libfuzzer_sys::fuzz_target;
use std::path::Path;
use vaa::task::parse_task_toml;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let _ = parse_task_toml(Path::new("fuzz.vaa.toml"), text);
});
