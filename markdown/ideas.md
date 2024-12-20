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
- [x] handle receiving new blocks by first verifying the transaction inside the block
- [x] delete mined transactions from the pool
    - [x] after mining a new block, delete the transaction from the pool
    - [x] after receiving a new mined block and validating it, removing the containing transaction from the pool if its there.
- [x] custom error data type for validating blocks
- [x] custom error data type for validating transactions
- [x] correct the order of pattern-matching when validating a new block wrt the local chain
   - [ ] handle each pattern-match correctly
        - [x] implementing merging fork suffixes
            - [x] test merging a valid fork suffix
            - [x] test merging an invalid fork suffix
        - [x] make chain struct contain a _private_ vector of blocks
        - [x] introduce additional data structures for tracking alternative chains and blocks
        - [x] change Chain::choose_chain to return an informative result as a custom data type
        - [ ] handle new blocks by:
            - [x] extending the main chain,
            - [x] adding new (single-block) forks to the main chian
            - [x] extending an existing chain from the main chain,
            - [x] adding new (single-block) forks to existing forks, but representing this as its own new fork from the main chain
            - [x] implement `sync_to_fork` for a Vec<Block>
            - [x] implement `sync_to_fork` for an entry `<forkpoint, endpoint>` stored in the pool
            - [x] perhaps modularise  `sync_to_fork`  for a Vec<Block> as instead:
                1. storing the fork as an entry in the pool,
                2. syncing to that fork in the local pool.
            - [x] to do: add tests for
                - [x] invalid forks (i.e. with missing parents)
                - [x] valid forks
                - [x] merging remote forks as Vec<Block>
                        - [x] shorter forks
                           - [x] produces correct ChooseChain result
                           - [x] updates the state of the Chain and its forks correctly
                        - [x] longer forks
                           - [x] produces correct ChooseChain result
                           - [x] updates the state of the Chain and its forks correctly
                - [x] merging local forks in the pool
            - [x] to do: automate (maybe not, due to disrupting current tests) or provide an explicit command for switching from the main chain to a longer fork
            - [x] to do: handle MissingBlocks in peer
                - [x] implement requesting new blocks until forming a valid fork suffix
                        - use a FIFO data structure
            - [x] to do: avoid doing anything for duplicate blocks
                        - introduce NextBlockRes::Duplicate
            - [ ] to do: avoid cloning blocks from an existing fork in order to represent the new fork
                        - possibly keep a hashset of all forks' blocks, and represent forks as vectors of references to blocks.
                        - above isnt possible: self-referentiality isnt allowed
                        - idea: keep a hashmap, or slotmap (which is apparently more efficient), of blocks. then represent chain and forks as vectors of strings, each one being a key to the hashmap
            - [x] to do: perform all validation of forks, orphans, chains, etc more concisely in a self-contained location, avoiding repeated validation.
                        - idea:
                        1. implement a Subchain struct, which can only store non-empty valid vectors of blocks. Instances of subchain are always valid.
                        2. then, use the Subchain as the fields in Chain, Forks, and Orphans.
            - [x] after swapping the main chain, revalidate all forks in the pool and delete nonvalid ones

                   [1] -[2] -[3] -[4] -[5] -[6] -[14]       Chain 1
                    |
                    |          *save at 9*
                    | -[7] -[8] - [9] -[10]-[11]            Chain 2
                               |
                               | -[12]-[13]                 Chain 3

            - [ ] make non-public the Chain functions that take an explicit (fork_hash: String,end_hash: String,)
            - [ ] more modular error messages, moving some out of NextBlockErr (use Dynamic error types)
            - [ ] implement validate for forks off of any chain

in parallel:
- [ ] data structure for storing new block proposals and number of validations, before adding it to the chain
- [ ] messages for sending new blocks proposals
- [ ] messages for responding with signed validations of blocks
- [ ] handling `n` peer validations of blocks by adding block to current chain

also:
- [ ] data for storing peers' public keys
- [ ] messages for sending public keys

also:
- [ ] optimise various functions to use lifetimes to return references
- [ ] optimise forks to avoid cloning overlapping blocks.
        - possibly keep a hashset of all forks' blocks, and represent forks as vectors of references to blocks.

----------------------

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

----------------------

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