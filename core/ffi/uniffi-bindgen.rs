//! The UniFFI bindings generator, built from the same uniffi version as the library
//! (`cargo run -p ouril-ffi --features cli --bin uniffi-bindgen -- generate …`).
fn main() {
    uniffi::uniffi_bindgen_main()
}
