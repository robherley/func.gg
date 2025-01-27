wit_bindgen::generate!({
  world: "run",
  path: "../../wit/funcgg.wit",
});

// https://github.com/bytecodealliance/wasi-rs

fn main() {
    funcgg::runtime::responder::set_header("X-Foo", "bar");
    funcgg::runtime::responder::set_status(200);

    println!("Environment variables:");
    for (key, value) in std::env::vars() {
        println!("{}: {}", key, value);
    }
}
