#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some((&kind, answer)) = data.split_first() else {
        return;
    };
    let Ok(answer) = std::str::from_utf8(answer) else {
        return;
    };
    common::prompt::fuzz_answer(kind, answer);
});
