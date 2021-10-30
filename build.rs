#[cfg(target_os = "linux")]
fn main() {
    cc::Build::new().file("constants.c").compile("ahaha");
}

#[cfg(not(target_os = "linux"))]
fn main() {
    /* No op */
}
