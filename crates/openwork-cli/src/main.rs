fn main() {
    if std::env::args().any(|argument| argument == "--version") {
        println!("OpenWork {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    println!("OpenWork bootstrap runtime");
}
