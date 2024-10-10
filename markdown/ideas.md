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
- [x] make command `mine [data]` with no `[data]` argument for mining a new block from the first transaction in the pool
- [ ] delete mined transactions from the pool
    - [ ] after mining a new block, delete the transaction from the pool
    - [ ] after receiving a new mined block, removing the containing transaction from the pool IF:
            1. the block.data can deserialize as a Transaction
            2. the block.data is also a valid Transaction

in parallel:
- [ ] data structure for storing new block proposals and number of validations, before adding it to the chain
- [ ] messages for sending new blocks proposals
- [ ] messages for responding with signed validations of blocks
- [ ] handling `n` peer validations of blocks by adding block to current chain

also:
- [ ] data for storing peers' public keys
- [ ] messages for sending public keys

also:
- [ ] research how to use **lifetimes** to return references

----
# Communication Algorithm

1. Transaction Broadcasting

    Originating Node: A node creates a new transaction (e.g., Alice sends 5 coins to Bob).
        Broadcasts: The node signs the transaction and broadcasts it to its connected peers.
    Receiving Nodes: All neighboring nodes receive this transaction.
        Verify: Each node verifies the transaction’s validity (e.g., checking signatures, ensuring Alice has enough balance).
        Re-Broadcast: If valid, the receiving nodes re-broadcast the transaction to their peers, propagating the transaction throughout the network.
    Storing: The verified transaction is stored in a local "transaction pool" or "mempool," waiting to be included in a new block.

2. Block Mining (Proof-of-Work Process)

    Miner Nodes: Nodes capable of mining (typically referred to as "miners") periodically check the transaction pool for new transactions.
        Select Transactions: The miner node selects a subset of unconfirmed transactions from its pool.
        Construct Block: The node constructs a new candidate block containing the selected transactions.
        Find Proof-of-Work: The node begins the Proof-of-Work process (finding a valid nonce that satisfies the difficulty condition).

3. Block Broadcasting

    Successful Miner: Once a miner successfully finds a valid nonce, it broadcasts the new block to its connected peers.
    Receiving Nodes: Each neighboring node receives the new block.
        Verify Block: Each node checks:
            The block’s Proof-of-Work (valid nonce).
            The validity of the included transactions (no double-spends, correct balances, etc.).
        Update Blockchain: If valid, the node updates its local blockchain by adding the new block.
        Re-Broadcast: The node re-broadcasts the new block to its peers, propagating it throughout the network.

4. Handling Conflicts (Fork Resolution)

    Chain Split: If a node receives a competing block (e.g., a different block mined at the same height), it stores it temporarily.
        Fork Handling: Nodes follow the longest-chain rule (or the heaviest-chain rule if defined differently). The chain with the most cumulative difficulty is considered the "main" chain.
    Chain Request: If the new block causes a fork or indicates that the node’s local chain is out-of-date, the node requests missing blocks from its peers to catch up.
        Synchronize: The node synchronizes its local blockchain by requesting and receiving missing blocks until it has the longest (or heaviest) chain.

5. Consensus and Propagation

    Consensus Achieved: Once a block is accepted and added to the local chain, the node considers the transactions in the block to be confirmed.
        Transaction Pool Update: Transactions that were part of the confirmed block are removed from the transaction pool.
        Broadcast Updates: Nodes may periodically broadcast their current block height or chain tip to indicate their progress to peers.

6. Handling New Transactions After a Block

    Receive New Transactions: Nodes continue receiving new transactions.
        Verify: Each transaction is verified individually and, if valid, added to the transaction pool.
    Start New Mining Round: Miners restart the mining process with the new pool of unconfirmed transactions.

7. Peer Discovery and Synchronization

    Discovery: Nodes periodically discover new peers and update their list of connected nodes.
    Chain Synchronization: New nodes joining the network typically request the latest block or chain state from peers to synchronize.

8. Recovery from Network Partitions

    Rejoin After Partition: If a network partition occurs (e.g., due to node disconnects or network failures), nodes that rejoin may need to re-sync their chains.
    Chain Requests: These nodes request the latest blocks from peers to catch up and rejoin the consensus.
----

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