use clap::Parser;
use once_cell::sync::Lazy;

pub static ARGS: Lazy<Args> = Lazy::new(Args::parse);

#[derive(clap::Parser)]
pub struct Args {
    #[arg(short, long, default_value_t = 8080)]
    pub port: u16,
}
