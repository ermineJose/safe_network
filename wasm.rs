use std::time::Duration;

// use libautonomi::Client;
//use tokio::time::sleep;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen_test::*;
// use sn_networking::target_arch::Instant;
use sn_networking::target_arch::spawn;
pub use wasmtimer::{
    std::Instant,
    tokio::{interval, sleep, timeout, Interval},
};

use web_sys::console; 
use log::Level;

mod common;

#[wasm_bindgen]
pub fn init_logging() {
    console_log::init_with_level(Level::Debug).expect("error initializing log");
    log::info!("Logging initialized.");
}

wasm_bindgen_test_configure!(run_in_browser);
// #[wasmtimer::test]
#[wasm_bindgen_test]
async fn file() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();
    log::info!("Helloworld");
    // Assert result and print test success
    // console::log_1(&"Test passed!".into());

    // common::enable_logging();

    // let peers = vec![
    //     "/ip4/127.0.0.1/tcp/35499/ws/p2p/12D3KooWGN5RqREZ4RYtsUc3DNCkrNSVXEzTYEbMb1AZx2rNddoW"
    //         .try_into()
    //         .expect("str to be valid multiaddr"),
    // ];

    // let mut client = Client::connect(&peers).await?;
    // log::info!("Client is initialized");
    // let mut wallet = common::load_hot_wallet_from_faucet();
    // let data = common::gen_random_data(1024 * 1024 * 10);

    // let addr = client.put(data.clone(), &mut wallet).await.unwrap();

    // sleep(Duration::from_secs(2)).await;

    // let data_fetched = client.get(addr).await.unwrap();
    // assert_eq!(data, data_fetched, "data fetched should match data put");

    Ok(())
}
