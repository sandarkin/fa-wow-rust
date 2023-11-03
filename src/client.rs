use anyhow::Result;
use std::error::Error;
use env_logger::Env;
use std::env;
use tcp_wow::{Client};

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let address = format!("{}:4444", host);
    let client = Client::new(address.as_str());
    match client.get_response() {
        Ok(r) => {
            log::info!("server response: {}", r);
            Ok(())
        }
        Err(e) => {
            log::error!("{}", e);
            Err(e)
        }
    }
}
