mod cli;
mod commands;
mod identity;
mod key_file;
mod workspace;

fn main() {
    if let Err(error) = commands::run() {
        eprintln!("sg-packager: {error}");
        std::process::exit(1);
    }
}
