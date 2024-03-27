mod lib;

#[cfg(not(any(target_arch = "wasm32", test)))]
fn main() {
    std::print!("{}", crate::lib::__get_candid_interface_tmp_hack());
}

#[cfg(any(target_arch = "wasm32", test))]
fn main() {}
