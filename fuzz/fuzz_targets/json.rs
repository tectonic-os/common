#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(value) = common::json::Json::parse(text) else {
        return;
    };
    let _ = common::json::Json::parse(&value.render());
});
