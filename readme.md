Small work-in-progress project, designing a peer-to-peer (P2P) network for blockchain.

### Running

Run the following (on multiple terminals) to initialise new peers on the same p2p network.

```sh
RUST_LOG=info cargo run --bin p2p
```

### Info

#### `peer.rs`:
The peer's logic on the local machine, which the entire application is architected around.

- Defines a peer id and public & private keys
- Initialises a swarm object for the peer, configured to the network behaviour
  - used to send local messages to the remote network
  - used to receive messages (and events) from the remote network, and forward these to the peer via a local channel
- Initialises a standard input buffer for the peer,
  - used to read console commands
- Defines the main (asynchronous) application loop
  - how to handle messages from the remote network (forwarded from the network behaviour object)
  - how to send messages to the remote network (via the swarm object)
  - how to handle commands from the console

#### `swarm.rs`:
The network logic for discovering peers, connecting to peers, sending/receiving messages, and handling network events.

Defines the interface for publishing request and response messages to the remote network
Defines a Swarm that executes the defined network behaviour and drives the entire network stack.
Defines a NetworkBehaviour that
  - Defines the types of messages transmitted in the network: responses and requests (for blocks).
  - Defines the type of our network which derives a `NetworkBehaviour`, which is used by all its peers.
  - Configures the communication and discovery protocols for the network
  - Defines how our network handles (behaves to) those protocol events.
    - For communication protocol, it receives request and response messages from remote peers, and
      - Forwards all relevant requests/responses to the local peer implementation.

#### `file.rs`:
Auxiliary data and functions relevant to the local machine.

- Defines interface for reading and writing blocks to local storage.

#### `block.rs`:
Core data and functions for blocks and chains.

- Defines types for Chains and Blocks.
- Defines interface for hashing, mining, validating, and choosing blocks and chains.


#### Architecture
```rs
                   ------------------------------> SWARM.rs ---------------------------->
                   ↑                                                                    |
                   |                                                                    |
                req/resp                                                             req/resp
                   |                                                                    |
                   |                                                                    ↓
  STDIN ====>  PEER.rs { chan_out } <=== req/resp ==== { chan_in }  SWARM.rs  <-- event <---   P2P_NETWORK
                   |                                           (network behaviour)
                   ↓
                FILE.rs
```

<!--
  Note:
  The Peer and NetworkBehaviour object never directly communicate. The Swarm is the intermediary that executes the one-way communication (the NetworkBehaviour sending messages to it the Peer via the local channel) describes in the code, when responding to events.
-->