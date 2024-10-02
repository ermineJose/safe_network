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

#[allow(clippy::unwrap_used)]
#[wasm_bindgen_test]
async fn fetch() -> Result<(), Box<dyn std::error::Error>> {
    console_error_panic_hook::set_once();

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false) // Only partially supported across browsers
        .without_time() // std::time is not available in browsers
        .with_writer(tracing_web::MakeWebConsoleWriter::new()); // write events to the console
    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(tracing_subscriber::EnvFilter::new(
            // Enable logging for crates and this file
            "sn_networking,autonomi,wasm",
        ))
        .init();

    let peers = vec![
        "/ip4/127.0.0.1/tcp/41135/ws/p2p/12D3KooWKPW8e4epFe6AWHuTKoWYGpq2egaCzbNNa6kzbBp6D6w6"
            .try_into()
            .expect("str to be valid multiaddr"),
    ];

    let client = Client::connect(&peers).await.unwrap();

    let addr = autonomi::client::address::str_to_xorname(
        "6425926e2044f3eacbbc3d4d34316295ac8b7e8ad753d99c358a7bf66d778d94",
    )?;

    let data_fetched = client.fetch_root(addr).await.unwrap();
    tracing::info!("{:?}", data_fetched);
    let file_pointer = data_fetched
        .map
        .get(&std::path::PathBuf::from("README.md"))
        .unwrap();

    let data_fetched = client.fetch_file(file_pointer).await.unwrap();
    tracing::info!("{}", String::from_utf8(data_fetched.to_vec()).unwrap());

    Ok(())
}
