----

# Top-Level PoW Algorithm

1. ## Joining a Network

    Chain Synchronization:
        Nodes that join or rejoin a network (e.g. for the first time, or due to network failures), sync or re-sync their local chain by requesting chains from all connected peers.

2. ## Transaction Broadcasting

    Originating Node: A node creates a new transaction (e.g., Alice sends 5 coins to Bob).
        Broadcasts: The node signs the transaction and broadcasts it to its connected peers.
    Receiving Nodes: All neighboring nodes receive this transaction.
        Verify:
        - Each node verifies the transaction’s validity (e.g., checking signatures, ensuring Alice has enough balance).
        - If valid,
           - The receiving nodes add a transaction to their local transaction pool immediately after they have personally verified it,  not after every other node has verified it. This transaction is waiting to be included in a new block.
           - The receiving nodes also re-broadcasts the transaction to their peers, propagating the transaction throughout the network.

3. ## Block Mining and Broadcasting

    **Miner Nodes**:
        Nodes capable of mining (typically referred to as "miners") periodically check the transaction pool for new transactions.
    **Select Transactions**:
        The miner node selects a subset of unconfirmed transactions from its pool.
    **Construct Block**:
        The node constructs a new candidate block containing the selected transactions.
    **Mining**:
        The node begins the Proof-of-Work process (finding a valid nonce that satisfies the difficulty condition).
    **Block Broadcasting:**
        Once a miner successfully finds a valid nonce:
        1. it immediately adds it to its local chain (without waiting for verification)
        2. it broadcasts the new block to its connected peers.

4. ## Block Receiving

    **Receiving Nodes:**
        Each neighboring node receives the new block.
        1. **Verify Block:**
            Each node checks:
            1. The validity of the block itself, e.g. correct index, hash, valid nonce.
            2. The validity of the each transaction (no double-spends, correct balances, etc.), regardless of whether it is in its local pool
        2. **Update Blockchain:**
            - **They are out-of-date**
                If the block is out-of-date, meaning the received block has a height less than the height of the current chain
            - **We are out-of-date**
                If the block indicates that the local chain is out-of-date, meaning the received block has a height index more than 2 than the local chain length,  or equivalently, if the block's parent block cannot be found in the chain.
                1. The node requests missing blocks from its peers to catch up, synchronizing its local blockchain by requesting and receiving missing blocks until it has the longest (or heaviest) chain.
            - **Both of us are up-to-date, but have diverged**
                If the block is at the same height as the most recent one:
                    - If it has the same parent, it is a **competing block**.
                      The node stores it temporarily.
                    - If it has a different parent but the same ancestor in the chain, it has caused a **fork**
                      The node requests the alternate chain starting from the fork point (common ancestor) to compare it with its current chain, and then chooses the longer chain
                    - If it has a different parent and no common ancestor, it is an **orphan**
                      This indicates a completely divergent chain.
                      The node may discard the block as invalid, as it cannot be integrated into its current state.
            - **If the block is valid**:
                1. The node updates its local blockchain by adding the new block.
                2. The node removes any confirmed transactions found in its pool (if any at all) that are present in the block.
        3. **Re-Broadcast:**
            The node re-broadcasts the new block to its peers, purely for the purpose of propagating it throughout the network.

5. ## General Block Broadcasting:

    - *Periodic Updates* Nodes may periodically broadcast their current block height (chain length) or chain tip to indicate their progress to peers, which other nodes can send back requests for if necessary
    - *Longest Chain Rule* Nodes follow the longest-chain rule (or the heaviest-chain rule if defined differently). The chain with the most cumulative difficulty is considered the "main" chain.

The process repeats: nodes continue receiving new transactions, each transaction is verified individually and, if valid, added to their local transaction pool, they restart the mining process with their new pool of unconfirmed transactions.

-----

## Block Receiving: Pattern Matching Algorithm

1. **Receive the new block B** and extract its parent hash `parent_hash_B`.

2. **Check if the block B is already in the local chain:**
   - If B is already in the chain:
     - Ignore the block (duplicate block).
     - Exit.

3. **Check if `parent_hash_B` is in the local chain:**
   - If `parent_hash_B` exists:
     - Identify the height of `parent_hash_B` in the local chain as `parent_height`.
     - Calculate `height_B = parent_height + 1`.

     - **Check if a block already exists at `height_B`:**
       - If no block exists at `height_B`:
         - Add B to the local chain as the new head.
         - Broadcast B to peers.
         - Exit.

       - If a block `current_block` already exists at `height_B`:
         - Check the parent hash of `current_block`:
           - If `current_block.parent_hash == parent_hash_B`:
             - This means B and `current_block` are competing blocks at the same height.
             - Keep both B and `current_block` in the local pool.
             - Mark B as a candidate block.
             - Exit.

           - If `current_block.parent_hash != parent_hash_B`:
             - This means B introduces a fork:
               - Store the forked branch (starting with B) in the local pool.
               - Evaluate if the new branch is a better chain using your consensus rule (e.g., longest chain, most cumulative work).
               - If the new branch is better:
                 - Switch to the forked branch and reorganize the chain.
                 - Broadcast the new head to peers.
               - If the current branch is better:
                 - Do not switch and keep track of the forked branch for future evaluation.
               - Exit.

   - If `parent_hash_B` is **NOT** in the local chain:
     - This means B references a parent block that the node does not have (missing parent).
     - Request the missing parent block from peers.
     - Store B in the pool of orphaned blocks.
     - Exit.

4. **Check if B is the new longest chain head:**
   - If the new block extends the chain and results in a longer chain:
     - Update the local chain to use B as the new head.
     - Broadcast B to peers.

```
START → Receive new block `B`
           ↓
Is `B` already in local chain? ---- YES ----> Ignore and Exit
           ↓
NO → Check if `B.parent_hash` exists in the chain:
           ↓
    ┌─────────────┬──────────────┐
    │             │              │
YES (Parent)    NO (Missing)   Parent exists but same height
    │             ↓              ↓
  Calculate    Store in      Competing blocks
 `height_B`    orphan pool
   (parent + 1)
    ↓
Height `height_B` already has a block?
    ↓
YES ───────────────────────→ Fork Detected
                            Evaluate Consensus:
    ↓                       ┌──────────────────────────┐
NO - Add `B` to chain      New branch is better?   Current is better?
     → Broadcast            └──────────────────────────┘
                             Switch, reorganize       Keep track for
                             and broadcast             future resolution
                               |                        |
                             Exit                      Exit
```