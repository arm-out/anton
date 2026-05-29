fn main() {
    if std::env::args().nth(1).as_deref() == Some("bench") {
        anton::benchmark::run();
    } else {
        anton::uci::run();
    }
}
