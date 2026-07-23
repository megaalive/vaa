#![no_main]

use libfuzzer_sys::fuzz_target;
use vaa::TransparencyDocument;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let _ = serde_json::from_str::<TransparencyDocument>(text);
});
