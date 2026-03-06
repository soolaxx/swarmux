fn main() {
    if let Err(error) = swarmux::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
