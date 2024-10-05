Work-in-progress project, designing a peer-to-peer (P2P) network for blockchain.

### Running

Run the following (on multiple terminals) to initialise new peers on the same p2p network.

```sh
RUST_LOG=info cargo run --bin main
```
### Commands
```
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
│     • Mine and add a new block to the chain, broadcasting this to other peers.

  *Request chain from peers*:
└── Usage: `req <"all" | [peer-id]>`
┌── Options:
│     • `"all"`      - Request chain from all peers
│     • `[peer-id]`  - Request chain from a specific peer

  *Print chain or peers*:
└── Usage: `show <"peers" | "chain">`
┌── Options:
│     • `"peers"`   - Show a list of discovered and connected peers
│     • `"chain"`   - Show current chain

  *Redial*:
└── Usage: `redial`
┌── Description:
│     • Redial all discovered peers.

  *Show commands*:
└── Usage: `help`
┌── Description:
│     • Prints this list of commands.
─────────────────────────────────────────────
```
### Files

#### `peer.rs`:
The peer's logic on the local machine, which the entire application is architected around.

Defines:
  - A Swarm object for the peer, used to send and receive messages to/from the remote network.
  - A standard input buffer for the peer, used to read console commands.
  - The main (asynchronous) application loop:
    - How to handle messages from the remote network (forwarded from the NetworkBehaviour object).
    - How to send messages to the remote network (via the Swarm object).
    - How to handle commands from the console.

#### `swarm.rs`:
The network logic for discovering peers, connecting to peers, sending/receiving messages, and handling network events.

Defines:
  - The Peer ID and public & private keys.
  - The interface for publishing request and response messages to the remote network.
  - The Swarm that executes the defined NetworkBehaviour.
  - The NetworkBehaviour for specific communication and discovery protocols, which specifies how those protocols' events should be handled. For the communication protocol, it receives request and response messages from remote peers and forwards all relevant requests/responses to the local peer application.

#### `message.rs`:
Types of potential messages transmitted in a network.

#### `block.rs`:
Core data and functions for blocks and chains.

Defines:
  - Types for Chains and Blocks.
  - The interface for hashing, mining, validating, and choosing blocks and chains.

#### `file.rs`:
Auxiliary data and functions relevant to the local machine.

Defines:
  - The interface for reading and writing blocks to local storage.


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