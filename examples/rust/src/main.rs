wit_bindgen::generate!({
  world: "run",
  path: "../../wit/funcgg.wit",
});

// https://github.com/bytecodealliance/wasi-rs

fn main() {
    funcgg::function::responder::set_header("X-Foo", "bar");
    funcgg::function::responder::set_header("Content-Type", "application/json");
    funcgg::function::responder::set_status(201);

    println!("{{");
    for (i, c) in ('a'..='z').enumerate() {
        std::thread::sleep(std::time::Duration::from_millis(500));
        println!("  \"{}\": \"{}\",", i, c);
    }
    println!("}}");
}
