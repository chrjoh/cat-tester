use clap::Parser;
mod client;
mod token;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Key to use, default work with the online checker https://cta-token.net/
    #[arg(
        short,
        long,
        default_value_t = String::from("403697de87af64611c1d32a05dab0fe1fcb715a86ab435f1ec99192d79569388")
    )]
    key: String,

    /// exp to add in renewal, token expier is set to ttl times two
    #[arg(long, default_value_t = 20)]
    ttl: u64,

    /// Select what type of token
    #[arg(value_enum,short,long,default_value_t = token::TokenType::Cookie)]
    token_type: token::TokenType,

    /// m3u8 url that return streaming segments
    #[arg(short, long)]
    url: String,

    /// token issuer to use
    #[arg(short,long,default_value_t=String::from("eyevinn"))]
    issuer: String,

    /// Number of times to fetch the segment
    #[arg(short, long, default_value_t = 5)]
    max_iterations: u32,

    /// time in ms to sleep between fething stream segment
    #[arg(long, default_value_t = 4000)]
    sleep: u64,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let worker = client::Worker::new(
        &args.key,
        &args.url,
        args.ttl,
        args.token_type,
        &args.issuer,
        args.max_iterations,
        args.sleep,
    );
    match worker.run().await {
        Ok(_) => println!("Worker completed all requests"),
        Err(e) => eprintln!("Worker failed: {}", e),
    }
}
