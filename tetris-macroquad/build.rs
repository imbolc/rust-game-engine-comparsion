fn main() {
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        // quad-snd's audio functions are supplied by the bundled JavaScript loader.
        println!("cargo::rustc-link-arg=--import-undefined");
    }
}
