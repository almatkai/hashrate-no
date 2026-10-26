#[tokio::main(worker_threads = 2)]
async fn main() {
    let result = match hashrate_no_native::Adapter::production() {
        Ok(adapter) => tezbar_extension_sdk::serve(adapter).await,
        Err(error) => Err(error),
    };
    if result.is_err() {
        // Do not print transport/library errors: they may contain request URLs.
        eprintln!("Hashrate process failed");
        std::process::exit(1);
    }
}
