## Architecture

/*************************************************************************************************************
                   -------------------------------------------> SWARM ---------------------------->
                   ↑                                                                               |
                   |                                                                               |
           request/response                                                                   request/response
                   |                                                                               |
                   |                                                                               ↓
 STDIN ==>       PEER                                       LOCAL NETWORKBEHAVIOUR <-- event <--- P2P NETWORK
           { LOCAL_RECEIVER } <========== request ===========  { LOCAL_SENDER }
                   ↑
                   ↓
               LOCAL IO
**************************************************************************************************************/

- peer.rs:
  - ...
- local_network.rs:
  - receives messages from remote peers
- local_swarm.rs:
  - ...
- local_data.rs:
  - ...

#### Running

```sh
RUST_LOG=info cargo run
```

#### Info

A peer-to-peer (P2P) network in which interconnected nodes ("peers") share resources
amongst each other without the use of a centralized administrative system.

[https://blog.logrocket.com/libp2p-tutorial-build-a-peer-to-peer-app-in-rust/]

