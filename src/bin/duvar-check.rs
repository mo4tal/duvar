fn main() {
    if std::env::args().any(|a| a == "--migrate") {
        match duvar_native::migrate_downloads() {
            Ok(n) => println!("Taşınan eski Duvar JSON dosyası: {n}"),
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        return;
    }
    let report = if std::env::args().any(|a| a == "--new-sources") {
        duvar_native::check_new_sources()
    } else {
        duvar_native::check_sources()
    };
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
