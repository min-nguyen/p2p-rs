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
    1. **Receive New Block**
      - Each neighboring node receives the new block.

    2. **Verify Block**
      - Each node checks:
        1. **The validity of the block's transactions**, regardless of whether it is in its local pool:
            - **Integrity**: Check if the hash is correct.
            - **Authenticity**: Verify if the signature is correct.
        2. **The validity of the block itself**, regardless of whether it can fit into our local chain:
            - **Pow Difficulty**: Ensure the hash meets the required difficulty prefix.
            - **Integrity**: Confirm that the hash is the actual computed hash of the rest of the block.

    3. **Update Blockchain**
      - Each node checks if the new block, confirmed to be valid, can extend the current chain:
        - **They are out-of-date**:
          - If the received block has a height less than the height of the current chain:
            - The block is either:
              - A **duplicate** of an older block (can be discarded).
              - An **orphan**—a different block at the same height of an older block (can be discarded).
        - **Both of us are up-to-date, but have diverged**:
          - If the block is at the same height as the most recent one:
            - If it has the same parent, it is a **competing block**; store it temporarily.
            - If it has a different parent, it belongs to a **forked chain** or a **divergent chain**:
              - Disregard the alternative chain (same height as current chain) or request the alternative chain back to the fork point, temporarily storing it for later use if the next valid block extends it.
        - **We are out-of-date by exactly one block**:
          - If the block is at a height one more than our current chain length:
            - If its parent matches our last block, it **directly extends** our chain and can be added, thus updating our chain.
            - If its parent does not match our last block, it belongs to a **forked chain**, requiring us to request blocks back to the fork point and replacing a suffix of our chain with it.
        - **We are out-of-date by more than one block**:
          - If the received block has a height index more than 2 greater than the local chain length:
            - The node requests missing blocks to catch up, working backwards until reaching the local current block.
            - Once receiving the final block {idx + 1}:
              - If block {idx + 1}'s parent matches our current block, then it **directly extends** our chain and can be added, thus updating our chain.
              - If block {idx + 1}'s parent does not match our current block, it belongs to a **forked chain**, requiring us to request blocks back to the fork point and replacing a suffix of our chain with it.

    4. **Update Transaction Pool and Re-Broadcast Received New Blocks**
      - For any new blocks added to our chain:
        - The node removes any confirmed transactions found in its pool (if any) that are present in the blocks.
        - The node re-broadcasts the new block to its peers for the purpose of propagating

    ### Requesting Missing Blocks
    When a new block references a parent that is not found in the local chain, the general approach is to recursively request the missing parent blocks until a common ancestor is located.

5. ## General Block Broadcasting:

    - *Periodic Updates* Nodes may periodically broadcast their current block height (chain length) or chain tip to indicate their progress to peers, which other nodes can send back requests for if necessary
    - *Longest Chain Rule* Nodes follow the longest-chain rule (or the heaviest-chain rule if defined differently). The chain with the most cumulative difficulty is considered the "main" chain.

The process repeats: nodes continue receiving new transactions, each transaction is verified individually and, if valid, added to their local transaction pool, they restart the mining process with their new pool of unconfirmed transactions.

-----

## Block Receiving Pattern Matching Algorithm

```
START → Receive new block `B`
           ↓
Is `B` valid?
           ├─── YES ────────→ Proceed to Update Blockchain
           ↓
          NO
           ↓
 Ignore `B` and Exit

           ↓
Is `B` out-of-date?
           ├─── YES ────────→ Discard
           ↓
          NO
           ↓
Is `B` at same height as current block?
           ├─── YES ────────────→
           │                        └─── Does `B.parent_hash` match current block hash?
           │                                        ├─── YES ──────→ Competing Block
           │                                        ↓
           │                                      NO
           │                                        ↓
           │                                Forked Chain
           │                                        ↓
           │                    Disregard or store for future use
           │
           ↓
         NO
           ↓
Is `B` height equal to current height + 1?
           ├─── YES ────────────→
           │                        └─── Does `B.parent_hash` match current block hash?
           │                                        ├─── YES ──────→ Add `B` to chain
           │                                        ↓
           │                                      NO
           │                                        ↓
           │                                Forked Chain
           │                                        ↓
           │                    Request blocks back to fork point and replace suffix
           │
           ↓
         NO
           ↓
Is `B` height greater than current height + 1?
           ├─── YES ─────────────→
           │                        ↓
           │              Request missing blocks to catch up
           │                        ↓
           │               Get final block
           │                        ↓
           │   Does final block parent match current block?
           │                        ├─── YES ──────→ Add `B` to chain
           │                        ↓
           │                      NO
           │                        ↓
           │                Forked Chain
           │                        ↓
           │        Request blocks back to fork point and replace suffix
           │
           ↓
         Exit
```