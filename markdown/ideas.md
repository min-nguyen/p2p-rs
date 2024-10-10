- [x] data types for generating, signing, and verifying transactions
- [x] messages for new transactions
- [x] temporary command `txn` for storing a test transaction in the pool
- [x] command `show txns` to show transaction pool
- [x] change Transaction fields `sender_pbk` and `sig` from type `Vec<u8>`  as hex strings for consistency
- [x] implement utility for encoding/decoding from hex strings to hashs and public keys
- [x] integrate above utility into chain.rs and transaction.rs
- [x] implement tests for valid and invalid transactions
- [x] change command `txn` for broadcasting new transactions
- [x] handling new transactions by storing them in the pool
- [ ] extend command `mine`  for mining new blocks based on transactions in the pool
- [ ] messages for proposing new blocks
- [ ] data for storing number of validations for a given block
- [ ] handling `n` peer validations of blocks by adding block to current chain
also:
- [ ] data for storing peers' public keys
- [ ] messages for sending public keys
- [ ] messages for responding with signed validations of blocks
also:
- [ ] research how to use **lifetimes** to return references

 +-----------------------+                   +-----------------------+
 |     Node (Miner 1)    |                   |     Node (Miner 2)    |
 | - Receives Transactions|                  | - Receives Transactions|
 | - Solves PoW Puzzle    |<---------------->| - Solves PoW Puzzle    |
 | - Broadcasts NewBlock  |                  | - Broadcasts NewBlock  |
 +-----------------------+                   +-----------------------+
           ^                                         ^
           |                                         |
           |     (Gossipsub: NewBlock + Tx)          |
           v                                         v
 +---------------------------------------------------------------+
 |                           Network                             |
 | - Gossipsub Topic for Broadcasting NewBlocks & Transactions   |
 | - mDNS for Peer Discovery                                      |
 +---------------------------------------------------------------+
           ^                                         ^
           |                                         |
           v                                         v
 +-----------------------+                   +-----------------------+
 |   Node (Validator 1)  |                   |   Node (Validator 2)  |
 | - Listens for NewBlocks|                  | - Listens for NewBlocks|
 | - Verifies PoW         |<---------------->| - Verifies PoW         |
 | - Updates Ledger       |                  | - Updates Ledger       |
 +-----------------------+                   +-----------------------+


1. Gossipsub with Reliable Transport

    Behavior: Use the Gossipsub protocol for broadcasting transactions and blocks while integrating a reliable transport layer (e.g., TCP with Noise for encryption).
    Purpose: This combination allows for efficient message propagation across peers with added security, ensuring that all nodes receive new blocks or transactions in a timely manner while keeping the data encrypted.

3. Identify Protocol with Gossipsub

    Behavior: Use the Identify Protocol to gather metadata about peers (like public keys and capabilities) and combine it with Gossipsub for message propagation.
    Purpose: By knowing the public keys and capabilities of peers, you can filter messages more intelligently, allowing only those from trusted peers to participate in consensus or block validation.

4. Transaction Pool with Gossipsub and Request/Response

    Behavior: Use a transaction pool behavior that listens to incoming transactions via Gossipsub and responds to requests for pending transactions.
    Purpose: This setup allows nodes to keep a local pool of unconfirmed transactions and serve them to other peers on request, reducing the burden of retransmitting data.

5. Request/Response for State Queries

    Behavior: Combine Request/Response protocol for querying the state of the blockchain (e.g., block headers, transaction details) with Gossipsub for broadcasting new blocks.
    Purpose: This allows peers to efficiently request specific data while also ensuring they remain updated with the latest blocks being propagated.

6. Custom Validation with Gossipsub and State Management

    Behavior: Create a custom validation behavior that listens for new blocks and transactions via Gossipsub while also managing local state updates.
    Purpose: This setup allows nodes to validate transactions and blocks before updating their local state, ensuring that only valid data is added to their blockchain.

7. Metrics Collection and Monitoring

    Behavior: Implement a separate behavior that collects metrics on network health (like latency, number of peers, message counts) while combining it with the standard blockchain behaviors.
    Purpose: This allows for real-time monitoring and analysis of network performance and can trigger alerts or adjustments to the network configuration based on the data collected.

8. Cross-Chain Communication

    Behavior: Use a combination of Gossipsub and custom request/response protocols to allow communication between different blockchains (cross-chain).
    Purpose: This allows your blockchain to interact with others, sharing information such as transaction proofs or confirmations while maintaining unique consensus mechanisms for each chain.

9. Incentivized Peer Participation

    Behavior: Implement a behavior that tracks peer contributions (like block validations or transactions relayed) and rewards them based on their activity.
    Purpose: This promotes active participation in the network and can be integrated with smart contracts to automate reward distribution.

10. Hierarchical Network Structure

    Behavior: Combine different behaviors for node tiers, such as full nodes that validate blocks and lightweight nodes that only participate in Gossipsub.
    Purpose: This allows for scalability, where lightweight nodes can act as relays without needing to store the entire blockchain, thus optimizing resource usage.