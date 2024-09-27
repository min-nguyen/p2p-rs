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

Run the following (on multiple terminals) to initialise new peers on the same p2p network.

```sh
RUST_LOG=info cargo run
```

#### Info

Small project designing a peer-to-peer (P2P) network.