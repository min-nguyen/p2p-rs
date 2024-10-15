
# Blockchain Fundamentals [https://www.oreilly.com/library/view/hands-on-smart-contract/9781492086116/ch01.html]

Blockchain is a distributed, decentralized, and immutable digital ledger that records data in a series of blocks, each cryptographically linked to the previous one, forming a chain. It enables secure, transparent, and decentralized record-keeping, eliminating the need for intermediaries.

## Networks vs Blockchains vs Cryptocurrencies
The terms network, a blockchain, and cryptocurrency are often used interchangeably to describe applications like Ethereum Classic and Bitcoin.

Strictly speaking:
  - A **network** is set of machines that work as a system of connected nodes that communicate to share resources.
  - A **blockchain** is the database --- a distributed ledger --- that a computer network manages, which has the information about various **data** and **transactions**
  - A **cryptocurrency** is a form of money that can be managed inside a blockchain database.
    It is a specific application of the blockchain technology designed to facilitate *financial* transactions.

For example:
  - Ethereum Classic is a **network** because it is a system of machines, nodes, and a shared database called a blockchain. In particular, it is a public network and its software is open source so that anyone can audit and use it to participate in the system.
  - Ethereum Classic is a **blockchain** because its database contains a ledger with accounts and balances, where transactions are fully transmitted and form a fully replicated chain of blocks.
  - Ethereum Classic is a **cryptocurrency** because its ledger tracks a coin called ETC that is scarce, durable, costly to create, portable, divisible, fungible, and transferable, so it may be used for payments and as a store of value.
  - Ethereum Classic allows users on the network to run **smart contracts**.

## Blockchain: Core Concepts

1. **Distributed Network**
   - A distributed network spreads processing and data storage across multiple nodes that work together.

2. **Decentralized Network**
   - A decentralized network is where no single node or authority has complete control.

3. **Blockchain**
   - A **blockchain** is the database --- a distributed ledger --- that a decentralized network manages, which has the information about (1) **data** and (2) **transactions**. It is represented by a chain data structure.
   - A **blockchain network**  typically has one main chain agreed upon by the majority of nodes, being the longest chain with the most valid and up-to-date data.
      - **Distributed**: Every node maintains a copy of the entire blockchain within a P2P network.
      - **Decentralized**: The nodes communicate to validate and agree on adding new blocks to the chain.

34. **Transactions**
  - Transactions are a fundamental concept in blockchain, and captures details of an activity that has taken place on a blockchain.
  - Transactions are a way to interact with a blockchain.
  - Transactions are the only way to change the state of the blockchain.

4. **Ledgers**
   - Ledgers are records of **data** and **transactions**.
   - **Uses of Ledgers Include**:
     - **Cryptocurrency**: Ledgers record **balances** and financial transactions made within a specific cryptocurrency network (e.g., Bitcoin).
     - **Supply Chain Management**: Ledgers record goods and transactions that track the movement of goods.
     - **Healthcare**: Ledgers record medical records of patients and transactions that capture interactions between healthcare providers, patients, and institutions.

5. **Blocks**
   - **Blocks Contain**:
     1. **Block Header**:
        - **Universal Fields**:
          - **Version**: Protocol version of the blockchain.
          - **Previous Block Hash**: Hash of the previous block's header.
          - **Timestamp**: Approximate creation time of the block.
          - **Merkle Root (or Equivalent)**: Root hash summarizing all data in the block. Specifically, it is the hash value at the top of a Merkle tree (or equivalent data structure) used to summarize and verify the integrity of large sets of data. If a blockchain only contains e.g. one transaction per block, the concept of a merkle root might be less critical.
        - **Additional Fields Specific to Cryptocurrency based on Proof-Of-Work Consensus**:
          - **Difficulty Target**: Criteria for block validation.
          - **Nonce**: Value adjusted to achieve a valid block hash.
     2. **Block Content**:
        - Ledger data, used to form part of the blockchain's ledger.

6. **Consensus Mechanism**
   - Consensus is a trust mechanism for multiple network nodes to agree on a single version of the blockchain (ledger). Many consensus mechanisms for blockchain exist.

7. **Hashing**
   - Hash functions are a one-way cryptographic function that produces a string unique to some input data. In blockchain, hashes are created for each block's data to ensure its integrity, as well as the entire chain's integrity by linking to the previous block's hash.

8. **Digital Signatures**
   - Digital signatures use a private key to encrypt text and a public key to decrypt text.
   - In blockchain, signatures are used to sign the individual data of blocks to:
     - Authenticate that the data was generated by a sender.
     - Ensure the signed data has not been changed.

9. **Smart Contracts**
   - These are programmable contracts that automatically execute predefined conditions.


#  Blockchain: Cryptocurrency
In the application of Blockchain to Cryptocurrency:

### Representing Cryptocurrency
Cryptocurrency is represented concretely as financial transactions on the blockchain, specific to a certain cryptocurrency network, with each transaction detailing the transfer of ownership from one address to another.

The specific representation can vary based on the blockchain's architecture and consensus mechanism.
   1. **UTXO Model**: In blockchains like Bitcoin, coins are represented as Unspent Transaction Outputs (UTXOs). Each UTXO can be thought of as a separate coin that is stored on the blockchain. Users' balances are derived from the total value of their UTXOs.
   2. **Account-based Model**: In Ethereum, balances are stored in accounts. Each account has an address, and its balance is updated as transactions occur. The current state of all accounts and their balances is stored in the blockchain.

### Issuing (Generating) New Cryptocurrency
In blockchain systems, the creation of new coins "from thin air" is governed by the rules of the protocol. This process is commonly known as coin issuance or coin generation, and it usually happens in two main scenarios: mining rewards and staking rewards.

- In Proof of Work (PoW) systems, new coins are generated as block rewards for miners who successfully add a new block to the blockchain.
- In Proof of Stake (PoS) systems, new coins are generated as rewards for validators (instead of miners) who successfully create a block, receiving a staking reward consisting of newly created coins and transaction fees from the block.

Coin generation is not arbitrary; it is controlled by strict rules embedded in the blockchain's code. These include:

- Consensus Protocols: The method (PoW, PoS, DPoS, etc.) by which new blocks and coins are created.
- Issuance Schedule: The rate at which new coins are issued (e.g., Bitcoin’s halving every four years).
- Maximum Supply: Some cryptocurrencies have a maximum supply limit, while others may not (like Ethereum, which currently has no hard cap but is reducing issuance over time).

### Accessing Cryptocurrency on a Blockchain

#### 1. Public and Private Keys
- **Public Key**: Derived from the private key; acts like a bank account number for receiving funds.
- **Private Key**: A secret key that proves ownership of funds; acts like a password.

#### 2. Accessing Your Coins
##### Ownership Proof
- Coins are associated with public addresses on the blockchain.
- Private keys prove ownership of the coins tied to a specific address.

##### Signing Transactions
- To send coins, you create a transaction and sign it with your private key:
  1. **Create a Transaction**: Specify the amount and recipient.
  2. **Sign the Transaction**: Use your private key to create a digital signature.
  3. **Broadcast**: Send the signed transaction to the network for verification.

#### 3. Example Process
1. **Receiving Coins**: Funds are sent to your public address.
2. **Spending Coins**:
  - Create a transaction in your wallet.
  - Sign it with your private key.
  - Send it to the blockchain.

##### Wallets
- Blockchain Explorers: You can look up your balance and transaction history using blockchain explorers that manually look up the balance on the block chain itself. However, this is not necessary for most (e.g. everyday) users.
- Digital wallets: handle these queries seamlessly, allowing users to focus on managing their currency without needing to interact with the blockchain directly. Blockchain explorers serve as a backup tool for verification and transparency.
  - Local Wallets store your private keys and transaction data directly on your device (computer or mobile).
  - Remote wallets store your private keys on a server controlled by a third party (e.g., exchanges or web-based wallets).

# Consensus Algrotihm

A consensus algorithm is used in blockchain networks to validate transactions and hence agree on new blocks to be added to the chain.

## Proof-of-Work (PoW)

One possible consensus algorithm for cryptocurrency is **Proof-of-Work (PoW)**:

- **PoW Block Structure**:
  1. **Difficulty Target**:
     - A number of zeros that miners (nodes) are trying to obtain in their output hash.
     - This regulates the difficulty of mining new blocks and ensures blocks are added at a consistent rate.
  2. **Nonce**:
     - A value that is adjusted during the block validation or creation process.
     - This is used to generate a hash that meets the required criteria for adding the block to the blockchain.

- **PoW Algorithm**:
  1. **Cryptographic Puzzle**:
     - Miners select and verify transactions from a pool of unconfirmed ones and organize these into a new block.
     - They then solve a puzzle: finding a "version" of their block that produces a "valid hash".
       - A valid hash is any hash with a certain number of leading zeros specified by the difficulty target.
       - Miners do this by varying only their block's nonce, keeping the rest fixed, and then hashing the block.

  2. **Chain Growth**:
     - The first successful miner broadcasts their block to the network.
     - Other nodes in the network verify the block's validity, and the block is added to the chain.
     - The miner is rewarded with cryptocurrency and the block's transaction fees.

  3. **Chain Consensus**:
     - The network's "main chain" is the longest valid chain owned by any node.
     - The nodes must communicate with each other to determine this.
     - In case of a tie (e.g., two blocks found at the same time), the network waits until more blocks are mined, and one chain becomes longer.

## Proof-of-Stake

Proof-of-Stake (PoS) is a consensus mechanism used in blockchain networks to validate transactions, create new blocks, and add them to the chain.

- **PoS Algorithm**:
1. Nodes make transactions. The PoS algorithm puts all these transactions in a pool.
2. All the nodes contending to become validator for the next block raise a stake.
    - This stake is combined with other factors like ‘coin-age’ or ‘randomized block selection’ to select the validator.
3. The validator verifies all the transactions, endorsing them as legitimate, and groups it into a block which they publish.
    - Their stake still remains locked and the forging reward is also not granted yet.
    - This is so that the nodes on the network can ‘OK’ the new block.
4. If the block is ‘OK’-ed by other validators, the validator gets the stake back and the reward too.
   - Different PoS will use different methods for verfiying the block.
     In sharding, a shard block is a block that contains transactions specifically processed by a shard of validators.
     1. a validator verifies a transaction by adding it to a shard block.
     2. after a shard block is created, a group of 128 validators will checks the transactions within it
     3. once all validators have attested to the shard block, if at least 2/3rds of the validators agree that the transactions within it are valid, it is confirmed overall and added to the chian.
5. If the block is not verified by other validators on the network, the validator(s) responsible for attesting to the validity of block loses their stake.