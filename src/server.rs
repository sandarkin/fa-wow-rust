use std::error::Error;
use env_logger::Env;
use tcp_wow::{Server};
use tcp_wow::DEFAULT_DIFFICULTY;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let address = "0.0.0.0:4444";
    let quotes_file = "./quotes.txt";
    let mut server = Server::new_from_file(&quotes_file)?;
    server.set_difficulty(DEFAULT_DIFFICULTY);
    server.run(address)
}
