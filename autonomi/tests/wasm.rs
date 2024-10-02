#![cfg(target_arch = "wasm32")]

use std::time::Duration;

use autonomi::Client;
use test_utils::evm::get_funded_wallet;
use tokio::time::sleep;
use tracing_subscriber::prelude::*;
use wasm_bindgen_test::*;

mod common;

wasm_bindgen_test_configure!(run_in_browser);

#[tokio::test]
#[wasm_bindgen_test]
async fn file() -> Result<(), Box<dyn std::error::Error>> {
    console_error_panic_hook::set_once();

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false) // Only partially supported across browsers
        .without_time() // std::time is not available in browsers
        .with_writer(tracing_web::MakeWebConsoleWriter::new()); // write events to the console
    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(tracing_subscriber::EnvFilter::new(
            "sn_networking,autonomi,wasm",
        ))
        .init();

    tracing::info!("Starting test");

    let peers = vec![
        "/ip4/127.0.0.1/tcp/41135/ws/p2p/12D3KooWKPW8e4epFe6AWHuTKoWYGpq2egaCzbNNa6kzbBp6D6w6"
            .try_into()
            .expect("str to be valid multiaddr"),
    ];

    let client = Client::connect(&peers).await.unwrap();
    let wallet = get_funded_wallet();

    let data = common::gen_random_data(1024 * 1024 * 10);

    let addr = client.put(data.clone(), &wallet).await.unwrap();

    sleep(Duration::from_secs(2)).await;

    let data_fetched = client.get(addr).await.unwrap();
    assert_eq!(data, data_fetched, "data fetched should match data put");

    Ok(())
}
