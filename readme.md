##  Interactive Proof-of-Work Blockchain Network in Rust 🦀

An interactive app for simulating various interactions between peers on an evolving Proof-of-Work blockchain network.

#### Running

Start multiple instances of the application on separate terminals to initialize new peers within the same peer-to-peer network.

```sh
cargo run
```
<!-- (RUST_LOG=info cargo run --bin main) -->

#### Commands Overview
```sh
  *Load chain*:
└── Usage: `load [file_name?]`
┌── Description:
│     • Load a chain to the application from a specified file name, defaulting to the file name `blocks.json`.

  *Save chain*:
└── Usage: `save [file_name?]`
┌── Description:
│     • Save the main chain to a specified file name, defaulting to the file name `blocks.json`.

  *Reset blockchain*:
└── Usage: `reset`
┌── Description:
│     • Reset main chain to a single genesis block and delete existing forks.

  *Create new transaction*:
└── Usage: `txn [data]`
┌── Description:
│     • Create a (random) transaction with the amount set to the given data, adding it to the pool, and broadcasting it to other peers.

  *Mine new block*:
└── Usage: `mine [data?]`
┌── Description:
|     • If no arguments are provided:
|       -  mine a block containing the first transaction in the pool (if any), adding it to the chain, and broadcasting it to other peers.
│     • If an argument is provided:
|       -  mine a block containing the given data, adding it to the chain, and broadcasting it to other peers.

  *Request chain from peers*:
└── Usage: `req <all | [peer-id]>`
┌── Options:
│     • `all`      - Request chain from all peers and synchronise to the most up-to-date chain
│     • `[peer-id]`  - Request chain from a specific peer and synchronise to the most up-to-date chain

  *Show peers/chain/forks/transaction pool*:
└── Usage: `show <peers | chain | forks | txns>`
┌── Options:
│     • `peers`   - Show list of discovered and connected peers
│     • `chain`   - Show main chain
│     • `forks`   - Show current forks from the main chain
│     • `txns`    - Show transaction pool

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
- Manages a Swarm object (for communicating with peers in the network).
- Manages std input events (for command-line interactions).
- Manages a local Chain object (which it both adds new mined blocks to and synchronises with other peers' chains).
- Manages a local Transaction pool (which it may mine new blocks for).

#### `swarm.rs`
Contains the network logic using GossipSub as the communication protocol and Mdns as the peer discovery protocol.
- Configures PeerId, Keypair, and Topic(s) for the network.
- Sets up NetworkBehaviour (that defines how peer discovery and message events are handled).
- Sets up Swarm (that executes the NetworkBehaviour).

#### `chain.rs`
Defines the blockchain and Proof-of-Work consensus algorithm.
- Chain internals, which manages a main chain, a hashmap of forks, and orphan branches.
- Methods for accessing, mining, extending, and validating a chain's blocks with respect to other blocks, chains, or forks.

```sh
cargo test chain -- --no capture
```

#### `block.rs`
Provides the block and Proof-of-Work mining algorithm.
- Block internals.
- Methods for hashing, mining, and validating blocks.
- Result and error types from handling new blocks.

```sh
cargo test block -- --no capture
```

#### `fork.rs`
Auxiliary helpers for identifying and updating a pool of forks and orphan branches.

#### `transaction.rs`
Provides the transaction form.
- Transaction type.
- Methods for generating and validating transactions.

```sh
cargo test transaction -- --no capture
```

#### `message.rs`
Provides the message forms communicated between peers.
- Messages for requesting and responding with chains or new blocks.
- Messages for broadcasting new transactions.

#### `file.rs`
Provides auxiliary access to local storage.
- Functions for loading and saving the blockchain state (from `blocks.json`).

---

### Architecture
```rs
  STDIN ====>     PEER.rs    <=== req/resp ====>  SWARM.rs  <-- pub/sub ---> Remote Network
               _____↑_____       MESSAGE.rs                                [TOPIC "transactions"]
               |          |                                                [TOPIC "chain"]
               ↓          ↓
            FILE.rs    CHAIN.rs
                       ↓      ↓
                    FORK.rs BLOCK.rs
```

<!--
  Note:
  The Peer and NetworkBehaviour object never directly communicate. The Swarm is the intermediary that executes the one-way communication (the NetworkBehaviour sending messages to it the Peer via the local channel) describes in the code, when responding to events.
-->
