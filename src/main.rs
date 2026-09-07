mod commands;
mod config;
mod i18n;
mod modules;
mod tui;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        None => commands::tui_cmd().await,
        Some("--version" | "-V") => {
            println!("nyx {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some("--help" | "-h") => {
            print_help();
            Ok(())
        }
        Some(command) => {
            eprintln!(
                "[!] Unknown command '{command}'. nyx-tui only opens the dashboard — run 'nyx' without arguments."
            );
            std::process::exit(1);
        }
    };

    if let Err(error) = result {
        eprintln!("[!] {error:#}");
        std::process::exit(1);
    }
}

fn print_help() {
    println!(
        "nyx-tui v{} — unofficial VulnyX terminal dashboard (https://vulnyx.com)",
        env!("CARGO_PKG_VERSION")
    );
    println!();
    println!("  nyx            open the interactive dashboard");
    println!("  nyx --version  print the version");
    println!("  nyx --help     show this help");
    println!();
    println!("Everything else lives inside the dashboard: machine catalog, first-blood");
    println!("flags, writeups, and your leaderboard position — no account needed, just a");
    println!("username. Press `d` on a machine to open its download page in your browser.");
}
