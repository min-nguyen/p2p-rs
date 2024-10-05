Small work-in-progress project, designing a peer-to-peer (P2P) network for blockchain.

// To do:
 - [x] load and maintain the local chain in the application, rather than reading-writing to a local file
 - [x] handle broadcasting new blocks
 - [ ] handle receiving new blocks
 - [ ] handle new chain requests
 - [ ] handle new chain responses

### Running

Run the following (on multiple terminals) to initialise new peers on the same p2p network.

```sh
RUST_LOG=info cargo run --bin p2p
```
### Commands
```
─────────────────────────────────────────────
  *Load chain*:
└── Usage: `load`
┌── Description:
│     • Load a chain to the application from a (predefined) local file `blocks.json`.

  *Save chain*:
└── Usage: `save`
┌── Description:
│     • Save the current chain to a (predefined) local file  `blocks.json`.

  *Print a list*:
└── Usage: `ls <"peers" | "chain">`
┌── Options:
│     • `"peers"`   - Show a list of discovered and connected peers
│     • `"chain"`   - Show current chain

  *Reset blockchain*:
└── Usage: `reset`
┌── Description:
│     • Reset current chain to a single block.

  *Mine new block*:
└── Usage: `mine [data]`
┌── Description:
│     • Mine and add a new block to the chain, broadcasting this to other peers.

  *Request chain from peers*:
└── Usage: `req <"all" | [peer-id]>`
┌── Options:
│     • `"all"`      - Request chain from all peers
│     • `[peer-id]`  - Request chain from a specific peer

  *Redial*:
└── Usage: `redial`
┌── Description:
│     • Redial all discovered peers.
─────────────────────────────────────────────
```
### Files

#### `peer.rs`:
The peer's logic on the local machine, which the entire application is architected around.

Defines:
  - a swarm object for the peer, used to send and receive messages to/from the remote network
  - a standard input buffer for the peer, used to read console commands
  - the main (asynchronous) application loop:
    - how to handle messages from the remote network (forwarded from the network behaviour object)
    - how to send messages to the remote network (via the swarm object)
    - how to handle commands from the console

#### `swarm.rs`:
The network logic for discovering peers, connecting to peers, sending/receiving messages, and handling network events.

Defines:
  - the peer id and public & private keys.
  - the interface for publishing request and response messages to the remote network
  - the Swarm that executes the defined NetworkBehaviour.
  - the NetworkBehaviour for specific communication and discovery protocol, which specifies how those protocols' events
    should be handled. For communication protocol, it receives request and response messages from remote peers, and
    forwards all relevant requests/responses to the local peer application.

#### `message.rs`:
Types of potential messages transmitted in a network.

#### `block.rs`:
Core data and functions for blocks and chains.

Defines:
  - types for Chains and Blocks.
  - interface for hashing, mining, validating, and choosing blocks and chains.

#### `file.rs`:
Auxiliary data and functions relevant to the local machine.

Defines:
  - interface for reading and writing blocks to local storage.


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