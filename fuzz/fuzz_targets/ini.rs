//! INI parser + settings model + writer round trip (S-FILE-07).
#![no_main]
use libfuzzer_sys::fuzz_target;
use yamdv_core::ini::Ini;
use yamdv_core::settings::Settings;

fuzz_target!(|data: &[u8]| {
    let ini = Ini::parse(data);
    let s = Settings::from_ini(&ini);
    let text = ini.write();
    let again = Ini::parse(text.as_bytes());
    // Round trip: the settings read back must be identical.
    assert_eq!(Settings::from_ini(&again), s);
    let mut w = again;
    s.write_into(&mut w);
    let _ = w.write();
});
