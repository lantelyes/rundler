// This file is part of Rundler.
//
// Rundler is free software: you can redistribute it and/or modify it under the
// terms of the GNU Lesser General Public License as published by the Free Software
// Foundation, either version 3 of the License, or (at your option) any later version.
//
// Rundler is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
// See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with Rundler.
// If not, see https://www.gnu.org/licenses/.

use std::{
    net::{Ipv4Addr, SocketAddr, TcpListener},
    time::Duration,
};

use discv5::Enr;
use ethers::types::H256;
use libp2p::PeerId;
use rundler_network::{
    enr::EnrExt, Action, AppRequest, AppRequestId, AppResponse, Config, Event, GossipMessage,
    Network, PooledUserOpHashesRequest, PooledUserOpHashesResponse, PooledUserOpsByHashRequest,
    PooledUserOpsByHashResponse, ResponseErrorKind, Result, UserOperationsWithEntryPoint,
    MAX_OPS_PER_REQUEST,
};
use rundler_types::UserOperation;
use tokio::{sync::mpsc, task::JoinHandle};
use tracing_test::traced_test;

struct TestNetworkContext {
    handle: JoinHandle<Result<()>>,
    action_sender: mpsc::UnboundedSender<Action>,
    event_receiver: mpsc::UnboundedReceiver<Event>,
    enr: Enr,
}

fn unused_port() -> u16 {
    let addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 0);
    let listener = TcpListener::bind(addr).unwrap();
    listener.local_addr().unwrap().port()
}

async fn setup_network(bootnodes: Vec<Enr>, supported_mempools: Vec<H256>) -> TestNetworkContext {
    let enr_key = discv5::enr::CombinedKey::generate_secp256k1();
    let private_key = hex::encode(enr_key.encode());

    let config = Config {
        bootnodes,
        private_key,
        supported_mempools,
        listen_address: SocketAddr::new(Ipv4Addr::LOCALHOST.into(), unused_port()),
        ..Default::default()
    };

    let (action_sender, action_receiver) = mpsc::unbounded_channel();
    let (event_sender, event_receiver) = mpsc::unbounded_channel();

    let network = Network::new(config, event_sender, action_receiver)
        .await
        .unwrap();
    let enr = network.enr().clone();

    let handle = tokio::spawn(async move { network.run().await });

    TestNetworkContext {
        handle,
        action_sender,
        event_receiver,
        enr,
    }
}

async fn setup_node_pair(mempools: Vec<H256>) -> (TestNetworkContext, TestNetworkContext) {
    let bootnode = setup_network(vec![], mempools.clone()).await;
    let node = setup_network(vec![bootnode.enr.clone()], mempools.clone()).await;
    (bootnode, node)
}

async fn shutdown_node_pair(mut node0: TestNetworkContext, node1: TestNetworkContext) {
    shutdown(node1).await;

    match node0.event_receiver.recv().await {
        Some(Event::PeerDisconnected { .. }) => {}
        _ => panic!("Expected peer disconnected event"),
    }

    shutdown(node0).await;
}

async fn shutdown_nodes(nodes: Vec<TestNetworkContext>) {
    for node in nodes {
        shutdown(node).await;
    }
}

async fn wait_for_pair_connect(
    node0: &mut TestNetworkContext,
    node1: &mut TestNetworkContext,
) -> (PeerId, PeerId) {
    let peer0 = match node0.event_receiver.recv().await {
        Some(Event::PeerConnected { peer_id }) => peer_id,
        _ => panic!("Expected peer connected event"),
    };

    let peer1 = match node1.event_receiver.recv().await {
        Some(Event::PeerConnected { peer_id }) => peer_id,
        _ => panic!("Expected peer connected event"),
    };

    (peer0, peer1)
}

async fn shutdown(mut context: TestNetworkContext) {
    let _ = context.action_sender.send(Action::Shutdown);
    loop {
        if let Some(Event::ShutdownComplete) = context.event_receiver.recv().await {
            break;
        }
    }
    let _ = context.handle.await.unwrap();
}

#[tokio::test]
async fn test_shutdown() {
    let context = setup_network(vec![], vec![]).await;
    shutdown(context).await;
}

#[tokio::test]
#[traced_test]
async fn test_peer_connect() {
    let (mut bootnode, mut node) = setup_node_pair(vec![]).await;

    match node.event_receiver.recv().await {
        Some(Event::PeerConnected { .. }) => {}
        _ => panic!("Expected peer connected event"),
    }

    match bootnode.event_receiver.recv().await {
        Some(Event::PeerConnected { .. }) => {}
        _ => panic!("Expected peer connected event"),
    }

    shutdown_node_pair(bootnode, node).await;
}

#[tokio::test]
#[traced_test]
async fn test_req_resp_op_hashes() {
    let (mut bootnode, mut node) = setup_node_pair(vec![]).await;
    let (node_peer_id, bootnode_peer_id) = wait_for_pair_connect(&mut bootnode, &mut node).await;

    let mempool = H256::random();

    bootnode
        .action_sender
        .send(Action::Request {
            peer_id: node_peer_id,
            request_id: AppRequestId(0),
            request: AppRequest::PooledUserOpHashes(PooledUserOpHashesRequest {
                mempool,
                offset: 0,
            }),
        })
        .unwrap();

    let request_id = match node.event_receiver.recv().await {
        Some(Event::RequestReceived {
            peer_id,
            request_id,
            request,
        }) => match request {
            AppRequest::PooledUserOpHashes(r) => {
                assert_eq!(peer_id, bootnode_peer_id);
                assert_eq!(r.mempool, mempool);
                assert_eq!(r.offset, 0);
                request_id
            }
            _ => panic!("Expected pooled user op hashes request"),
        },
        _ => panic!("Expected request received event"),
    };

    let hashes = vec![H256::random(), H256::random()];

    node.action_sender
        .send(Action::Response {
            request_id,
            response: Ok(AppResponse::PooledUserOpHashes(
                PooledUserOpHashesResponse {
                    more_flag: true,
                    hashes: hashes.clone(),
                },
            )),
        })
        .unwrap();

    match bootnode.event_receiver.recv().await {
        Some(Event::ResponseReceived {
            peer_id,
            request_id,
            response,
        }) => match response {
            Ok(AppResponse::PooledUserOpHashes(r)) => {
                assert_eq!(peer_id, node_peer_id);
                assert_eq!(request_id, AppRequestId(0));
                assert!(r.more_flag);
                assert_eq!(r.hashes, hashes);
            }
            _ => panic!("Expected pooled user op hashes response"),
        },
        _ => panic!("Expected response received event"),
    }

    shutdown_node_pair(bootnode, node).await;
}

#[tokio::test]
#[traced_test]
async fn test_req_resp_ops_by_hashes() {
    let (mut bootnode, mut node) = setup_node_pair(vec![]).await;
    let (node_peer_id, bootnode_peer_id) = wait_for_pair_connect(&mut bootnode, &mut node).await;

    let hashes = vec![H256::random(), H256::random()];

    bootnode
        .action_sender
        .send(Action::Request {
            peer_id: node_peer_id,
            request_id: AppRequestId(0),
            request: AppRequest::PooledUserOpsByHash(PooledUserOpsByHashRequest {
                hashes: hashes.clone(),
            }),
        })
        .unwrap();

    let request_id = match node.event_receiver.recv().await {
        Some(Event::RequestReceived {
            peer_id,
            request_id,
            request,
        }) => match request {
            AppRequest::PooledUserOpsByHash(r) => {
                assert_eq!(peer_id, bootnode_peer_id);
                assert_eq!(r.hashes, hashes);
                request_id
            }
            _ => panic!("Expected pooled user op hashes request"),
        },
        _ => panic!("Expected request received event"),
    };

    node.action_sender
        .send(Action::Response {
            request_id,
            response: Ok(AppResponse::PooledUserOpsByHash(
                PooledUserOpsByHashResponse {
                    user_ops: vec![UserOperation::default(), UserOperation::default()],
                },
            )),
        })
        .unwrap();

    match bootnode.event_receiver.recv().await {
        Some(Event::ResponseReceived {
            peer_id,
            request_id,
            response,
        }) => match response {
            Ok(AppResponse::PooledUserOpsByHash(r)) => {
                assert_eq!(peer_id, node_peer_id);
                assert_eq!(request_id, AppRequestId(0));
                assert_eq!(r.user_ops.len(), 2);
            }
            _ => panic!("Expected pooled user op hashes response"),
        },
        _ => panic!("Expected response received event"),
    }

    shutdown_node_pair(bootnode, node).await;
}

#[tokio::test]
#[traced_test]
async fn test_req_resp_ops_by_hashes_too_many() {
    let (mut bootnode, mut node) = setup_node_pair(vec![]).await;
    let (node_peer_id, _) = wait_for_pair_connect(&mut bootnode, &mut node).await;

    let hashes = vec![H256::random(); MAX_OPS_PER_REQUEST + 1];

    bootnode
        .action_sender
        .send(Action::Request {
            peer_id: node_peer_id,
            request_id: AppRequestId(0),
            request: AppRequest::PooledUserOpsByHash(PooledUserOpsByHashRequest {
                hashes: hashes.clone(),
            }),
        })
        .unwrap();

    match bootnode.event_receiver.recv().await {
        Some(Event::ResponseReceived {
            peer_id,
            request_id,
            response,
        }) => match response {
            Err(e) => {
                assert_eq!(peer_id, node_peer_id);
                assert_eq!(request_id, AppRequestId(0));
                assert_eq!(e.kind, ResponseErrorKind::InvalidRequest);
            }
            _ => panic!("Expected pooled user op hashes response error"),
        },
        _ => panic!("Expected response received event"),
    };

    shutdown_node_pair(bootnode, node).await;
}

#[tokio::test]
#[traced_test]
async fn test_discovery() {
    let (mut bootnode, mut node0) = setup_node_pair(vec![]).await;
    wait_for_pair_connect(&mut bootnode, &mut node0).await;

    let mut node1 = setup_network(vec![bootnode.enr.clone()], vec![]).await;

    // node 1 should discover both bootnode and node0
    for _ in 0..2 {
        match node1.event_receiver.recv().await {
            Some(Event::PeerConnected { peer_id }) => {
                assert!(peer_id == bootnode.enr.peer_id() || peer_id == node0.enr.peer_id())
            }
            _ => panic!("Expected discovered peer event"),
        }
    }

    // node 0 should discover node 1
    match node0.event_receiver.recv().await {
        Some(Event::PeerConnected { peer_id }) => assert_eq!(peer_id, node1.enr.peer_id()),
        _ => panic!("Expected discovered peer event"),
    }

    shutdown_nodes(vec![bootnode, node0, node1]).await;
}

#[tokio::test]
#[traced_test]
async fn test_gossip() {
    let mempool = H256::random();
    let (mut bootnode, mut node0) = setup_node_pair(vec![mempool]).await;
    wait_for_pair_connect(&mut bootnode, &mut node0).await;

    bootnode
        .action_sender
        .send(Action::GossipMessage {
            mempool_ids: vec![mempool],
            message: GossipMessage::UserOperationsWithEntryPoint(UserOperationsWithEntryPoint {
                entry_point_contract: Default::default(),
                verified_at_block_hash: Default::default(),
                chain_id: Default::default(),
                user_operations: vec![UserOperation::default()],
            }),
        })
        .unwrap();

    match node0.event_receiver.recv().await {
        Some(Event::GossipMessage {
            id: _,
            peer_id,
            mempool_id,
            message,
        }) => {
            assert_eq!(peer_id, bootnode.enr.peer_id());
            assert_eq!(mempool_id, mempool);
            match message {
                GossipMessage::UserOperationsWithEntryPoint(uo) => {
                    assert_eq!(uo.user_operations.len(), 1);
                }
            }
        }
        _ => panic!("Expected gossip message received event"),
    }

    shutdown_node_pair(bootnode, node0).await;
}

#[tokio::test]
#[traced_test]
async fn test_gossip_topics() {
    let mempool0 = H256::random();
    let mempool1 = H256::random();

    let mut bootnode = setup_network(vec![], vec![mempool0, mempool1]).await;
    let mut node0 = setup_network(vec![bootnode.enr.clone()], vec![mempool0]).await;
    let mut node1 = setup_network(vec![bootnode.enr.clone()], vec![mempool1]).await;

    // TODO: unsure if there is as better way to do this without
    // surfacing more information.
    // This waits for the peers to connect, and then to subscribe to the gossip topics.
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Drain all the events thus far
    while bootnode.event_receiver.try_recv().is_ok() {}
    while node0.event_receiver.try_recv().is_ok() {}
    while node1.event_receiver.try_recv().is_ok() {}

    // Send a message across mempools
    bootnode
        .action_sender
        .send(Action::GossipMessage {
            mempool_ids: vec![mempool0, mempool1],
            message: GossipMessage::UserOperationsWithEntryPoint(UserOperationsWithEntryPoint {
                entry_point_contract: Default::default(),
                verified_at_block_hash: Default::default(),
                chain_id: Default::default(),
                user_operations: vec![UserOperation::default()],
            }),
        })
        .unwrap();

    // Mempool 0 should receive
    match node0.event_receiver.recv().await {
        Some(Event::GossipMessage {
            id: _,
            peer_id,
            mempool_id,
            message,
        }) => {
            assert_eq!(peer_id, bootnode.enr.peer_id());
            assert_eq!(mempool_id, mempool0);
            match message {
                GossipMessage::UserOperationsWithEntryPoint(uo) => {
                    assert_eq!(uo.user_operations.len(), 1);
                }
            }
        }
        _ => panic!("Expected gossip message received event"),
    }

    // TODO: libp2p will deduplicate this message before sending to the 2nd mempool topic.
    // This is an issue with how the spec is defined.
    // match node1.event_receiver.recv().await {
    //     Some(Event::GossipMessage {
    //         id: _,
    //         peer_id,
    //         mempool_id,
    //         message,
    //     }) => {
    //         assert_eq!(peer_id, bootnode.enr.peer_id());
    //         assert_eq!(mempool_id, mempool1);
    //         match message {
    //             GossipMessage::UserOperationsWithEntryPoint(uo) => {
    //                 assert_eq!(uo.user_operations.len(), 1);
    //             }
    //         }
    //     }
    //     _ => panic!("Expected gossip message received event"),
    // }
}
