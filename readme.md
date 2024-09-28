Small work-in-progress project, designing a peer-to-peer (P2P) network.

### Running

Run the following (on multiple terminals) to initialise new peers on the same p2p network.

```sh
RUST_LOG=info cargo run
```

### Info

#### `peer.rs`:
The peer's logic on the local machine, which the entire application is architected around.

- Defines a peer id and public & private keys
- Initialises a network behaviour object for the peer
  - used to receive messages (and events) from the remote network, and forward these to the peer via a local channel
- Initialises a swarm object for the peer, configured to the network behaviour
  - used to send local messages to the remote network
- Initialises a standard input buffer for the peer,
  - used to read console commands
- Defines the main (asynchronous) application loop
  - how to handle messages from the remote network (forwarded from the network behaviour object)
  - how to send messages to the remote network (via the swarm object)
  - how to handle commands from the console

#### `network.rs`:
Describes global configuration of protocols and messages supported by the network, necessary to all peers that access it.
Describes local behaviour of how network events are handled, common to all peers using this specific implementation.

- Defines the types of messages transmitted in the network: responses and requests (for blocks).
- Defines the type of our network which derives a `NetworkBehaviour`, which is used by all its peers.
  - Configures the communication and discovery protocols for the network
  - Defines how our network handles (behaves to) those protocol events.
    - For communication protocol, it receives request and response messages from remote peers, and
      - (Currently) simply logs all responses to the terminal
      - Forwards all relevant requests to the local peer implementation.

#### `swarm.rs`:
Drives the entire network stack, and executes the defined network behaviour.

- Defines interface for publishing request and response messages to the remote network

#### `file.rs`:
Auxiliary data and functions relevant to the local machine.

- Defines types for core data i.e. blocks.
- Defines interface for reading and writing core data to local storage.


#### Architecture
```rs
                    ------------------------------> SWARM.rs -------------------------->
                    ↑                                                                 |
                    |                                                                 |
                 req/resp                                                          req/resp
                    |                                                                 |
                    |                                                                 ↓
  STDIN ----->     PEER.rs                          NETWORK.rs  <-- event <---   P2P_NETWORK
                { chan_out } <==== request =====   { chan_in }
                    |
                    ↓
                  FILE.rs
```

<!-- The communication described between Peer and NetworkBehaviour (via the local channel), at run-time, is actually executed by the Swarm as an intermediary. -->