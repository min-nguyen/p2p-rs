##  Interactive Proof-of-Work Blockchain Network in Rust 🦀

An ongoing Rust-based project to build a decentralized Proof-of-Work blockchain network, with a command-line interface for interactions.

#### Running

Start multiple instances of the application on separate terminals to initialize new peers within the same peer-to-peer network.

```sh
RUST_LOG=info cargo run --bin main
```

#### Commands Overview
```sh
─────────────────────────────────────────────
  *Load chain*:
└── Usage: `load`
┌── Description:
│     • Load a chain to the application from a (predefined) local file `blocks.json`.

  *Save chain*:
└── Usage: `save`
┌── Description:
│     • Save the current chain to a (predefined) local file `blocks.json`.

  *Reset blockchain*:
└── Usage: `reset`
┌── Description:
│     • Reset current chain to a single block.

  *Mine new block*:
└── Usage: `mine [data]`
┌── Description:
│     • Mine a block with the given data, adding it to the chain, and broadcasting it to other peers.

  *Create new transaction*:
└── Usage: `txn [data]`
┌── Description:
│     • Create a (random) transaction with the amount set to the given data, adding it to the pool, and broadcasting it to other peers.

  *Request chain from peers*:
└── Usage: `req <"all" | [peer-id]>`
┌── Options:
│     • `"all"`      - Request chain from all peers
│     • `[peer-id]`  - Request chain from a specific peer

  *Show peers/chain/transaction pool *:
└── Usage: `show <"peers" | "chain" | "txns">`
┌── Options:
│     • `"peers"`   - Show a list of discovered and connected peers
│     • `"chain"`   - Show current chain
│     • `"txns"`    - Show transaction pool

  *Redial*:
└── Usage: `redial`
┌── Description:
│     • Redial all discovered peers.

  *Command menu*:
└── Usage: `help`
┌── Description:
│     • Prints this list of commands.
─────────────────────────────────────────────
```

---
### File Overview

#### `peer.rs`
Manages the core peer logic, providing the main application loop and interfaces for sending/receiving messages.
- Manages a Swarm object for peer communication within the network.
- Manages standard input events for command-line interactions.

#### `swarm.rs`
Contains the network logic using GossipSub as the communication protocol and Mdns as the peer discovery protocol.
- Configures PeerId, Keypair, and Topic(s) for the network.
- Sets up NetworkBehaviour to define how immediate peer discovery and message events are handled.
- Sets up Swarm that wraps around and executes the NetworkBehaviour.

#### `chain.rs`
Provides the blockchain data structures and the Proof-of-Work consensus algorithm.
- Defines Chain and Block types.
- Implements methods for hashing, mining, validating, and managing blocks.

#### `message.rs`
Specifies message formats used for communication between nodes in the network.
- Encodes request/response messages and broadcasts for new blocks.

#### `file.rs`
Handles reading and writing the blockchain data to and from local storage.
- Manages file operations for loading and saving the blockchain state (`blocks.json`).


---

### Architecture
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
