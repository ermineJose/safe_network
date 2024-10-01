use std::{collections::HashSet, time::Duration};

use libp2p::{identity::Keypair, Multiaddr};
use sn_client::networking::{
    multiaddr_is_global, version::IDENTIFY_PROTOCOL_STR, Network,NetworkClient, NetworkBuilder, NetworkEvent,
};

use sn_networking::NetworkBuilderClient;

// use crate::NetworkBuilderClient;
use sn_protocol::CLOSE_GROUP_SIZE;
// use tokio::{sync::mpsc::Receiver, time::interval};

mod data;
#[cfg(not(target_arch = "wasm32"))]
mod files;
mod registers;
mod transfers;

// use wasmtimer::timer:;

// use crate::NetworkBuilderClient;

// Time before considering the connection timed out.
const CONNECT_TIMEOUT_SECS: u64 = 20;

use tokio::sync::mpsc::Receiver;
pub use wasmtimer::{
    std::Instant,
    tokio::{interval, sleep, timeout, Interval},
};

pub use wasm_bindgen_futures::spawn_local as spawn;
use futures::channel::oneshot;
use futures::select;
use log::Level;

#[derive(Clone)]
pub struct Client {
    pub(crate) network: NetworkClient,
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    #[error("Could not connect to peers due to incompatible protocol: {0:?}")]
    TimedOutWithIncompatibleProtocol(HashSet<String>, String),
    #[error("Could not connect to enough peers in time.")]
    TimedOut,
}

impl Client {
    /// ```no_run
    /// # use libautonomi::Client;
    /// let peers = ["/ip4/127.0.0.1/udp/1234/quic-v1".parse().expect("str to be valid multiaddr")];
    /// let client = Client::connect(&peers);
    /// ```
    pub async fn connect(peers: &[Multiaddr]) -> Result<Self, ConnectError> {
        // Any global address makes the client non-local
        let local = !peers.iter().any(multiaddr_is_global);

        let (network, event_receiver) = build_client_and_run_swarm(local);

        // Spawn task to dial to the given peers
        let network_clone = network.clone();
        let peers = peers.to_vec();
        
        let _handle = spawn(async move {
            for addr in peers {
                if let Err(err) = network_clone.dial(addr.clone()).await {
                    eprintln!("addr={addr} Failed to dial: {err:?}");
                };
            }
        });
        //Sender<Result<(), ConnectError>>
        let (sender, receiver) = oneshot::channel();
        spawn(handle_event_receiver(event_receiver, sender));
        log::info!("spawn variable created");
        receiver.await.expect("sender should not close")?;
        log::info!("receiver await exit");
        Ok(Self { network })
    }
}

fn build_client_and_run_swarm(local: bool) -> (NetworkClient, Receiver<NetworkEvent>) {
    // TODO: `root_dir` i only used for nodes. `NetworkBuilder` should not require it.
    // let root_dir = std::env::temp_dir();
    let network_builder = NetworkBuilderClient::new(Keypair::generate_ed25519(), local);

    // TODO: Re-export `Receiver<T>` from `sn_networking`. Else users need to keep their `tokio` dependency in sync.
    // TODO: Think about handling the mDNS error here.
    let (network, event_receiver, swarm_driver) =
        network_builder.build_client().expect("mdns to succeed");

    let _swarm_driver = spawn(swarm_driver.run());

    (network, event_receiver)
}

async fn handle_event_receiver(
    mut event_receiver: Receiver<NetworkEvent>,
    sender: oneshot::Sender<Result<(), ConnectError>>,
) {
    log::info!("handle event receiver inside");
    // We switch this to `None` when we've sent the oneshot 'connect' result.
    let mut sender = Some(sender);
    let mut unsupported_protocols = vec![];

    let mut timeout_timer = interval(Duration::from_secs(CONNECT_TIMEOUT_SECS));
    log::info!("timeout_timer variable created");
    timeout_timer.tick().await;
    log::info!("timeout timer variable tick await");
    
    loop {
        futures::select! {
            _ = timeout_timer.tick() =>  {
                if let Some(sender) = sender.take() {
                    if unsupported_protocols.len() > 1 {
                        let protocols: HashSet<String> =
                            unsupported_protocols.iter().cloned().collect();
                        sender
                            .send(Err(ConnectError::TimedOutWithIncompatibleProtocol(
                                protocols,
                                IDENTIFY_PROTOCOL_STR.to_string(),
                            )))
                            .expect("receiver should not close");
                    } else {
                        sender
                            .send(Err(ConnectError::TimedOut))
                            .expect("receiver should not close");
                    }
                }
            }
            event = event_receiver.recv() => {
                let event = event.expect("receiver should not close");
                match event {
                    NetworkEvent::PeerAdded(_peer_id, peers_len) => {
                        tracing::trace!("Peer added: {peers_len} in routing table");

                        if peers_len >= CLOSE_GROUP_SIZE {
                            if let Some(sender) = sender.take() {
                                sender.send(Ok(())).expect("receiver should not close");
                            }
                        }
                    }
                    NetworkEvent::PeerWithUnsupportedProtocol { their_protocol, .. } => {
                        tracing::warn!(their_protocol, "Peer with unsupported protocol");

                        if sender.is_some() {
                            unsupported_protocols.push(their_protocol);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // TODO: Handle closing of network events sender
}
