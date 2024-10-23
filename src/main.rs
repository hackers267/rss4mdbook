use std::env;

mod inv;

fn main() {
    env::set_var("RUST_LOG", "info");
    env_logger::init();
    inv::run();
}
