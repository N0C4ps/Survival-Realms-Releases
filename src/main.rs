pub mod application;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    application::app::run()
}
