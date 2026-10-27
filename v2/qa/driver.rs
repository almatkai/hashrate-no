//! Owned TLS fixture transport. Excluded from release archives and default builds.
#[tokio::main(worker_threads = 2)]
async fn main() {
    let args: Vec<_> = std::env::args().collect();
    let port = args[1].parse().expect("owned port");
    let cert = std::fs::read(&args[2]).expect("owned certificate");
    tezbar_extension_sdk::serve(
        hashrate_no_native::Adapter::owned_tls_fixture(port, &cert).unwrap(),
    )
    .await
    .unwrap();
}
