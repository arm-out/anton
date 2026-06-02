fn main() {
    if std::env::args().nth(1).as_deref() == Some("bench") {
        let ci: bool = if std::env::args().nth(2).as_deref() == Some("--ci") {
            true
        } else {
            false
        };
        anton::benchmark::run(ci);
    } else {
        anton::uci::run();
    }
}
