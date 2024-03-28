mod lib;

#[cfg(not(any(target_arch = "wasm32", test)))]
fn main() {
    std::print!("{}", crate::lib::export_candid());
}

#[cfg(any(target_arch = "wasm32", test))]
fn main() {}
