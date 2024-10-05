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

---

### File Overview

##### `peer.rs`
Handles the logic for local peer nodes and serves as the foundation of the application.

- **Components**:
  - **Swarm Object**: Manages peer communication and message handling within the network.
  - **Standard Input Buffer**: Reads console commands and interprets them into application actions.
  - **Asynchronous Application Loop**: Processes messages from the network, sends commands, and handles input from the console.

#### `swarm_gossip.rs`
Implements the core networking logic using `GossipSub` for message exchange and `Mdns` for peer discovery.

- **Components**:
  - **PeerId and Keypair**: Establishes the identity of the node in the network.
  - **Topics and NetworkBehaviour**: Manages peer connections, event handling, and protocol communication.
  - **Swarm Instance**: Encapsulates the network behaviour and handles incoming/outgoing events and messages.

#### `block.rs`
Contains the blockchain logic, built around a simplified Proof-of-Work consensus algorithm.

- **Data Types**:
  - **`Chain`**: Represents the entire blockchain structure.
  - **`Block`**: Represents individual blocks in the chain.
- **Functions**: 
  - Methods for hashing, mining, validating, and managing blocks and chains.

#### `message.rs`
Defines the possible message types exchanged in the blockchain network.

- **Message Types**:
  - Requests for chain data.
  - Responses containing block information.
  - Notifications for newly mined blocks.

#### `file.rs`
Provides utilities for reading and writing blockchains to and from local storage.

- **Functions**:
  - Reading a blockchain from a file.
  - Writing the current chain state to a file for persistence.

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
