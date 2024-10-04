# Networks and Blockchains and Cryptocurrencies
When people talk about Ethereum Classic or Bitcoin they usually call them either a network, a blockchain, or a cryptocurrency interchangeably.

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

# BlockChain

A **blockchain** is the database --- a distributed ledger --- that a computer **network** manages, which has the information about various **data** and **transactions**
  - Its main purpose and underlying technology is to enable the secure and decentralized recording of **data** (e.g. accounts, balances, smart contracts of the system) and **transactions** (activities that have taken place on the block chain).
  - Decentralization: No single entity controls the data; all participants maintain a copy of the ledger.
  - Immutability: Once data is recorded, it’s extremely difficult to alter, ensuring data integrity.
  - Transparency: Transactions are visible to all participants, fostering accountability.
  - Security: Cryptographic techniques protect the data, making it resistant to fraud and unauthorized changes.

A **transaction** is a fundamental concept in the blockchain world, transaction captures details of an activity that has taken place on a blockchain.
  - Transactions are a way to interact with a blockchain.
  - Transactions are the only way to change the state of the blockchain.

# Cryptocurrency (Representation)

Cryptocurrency, or coins, are represented concretely as transactions on the blockchain, with each transaction detailing the transfer of ownership from one address to another. This structure ensures transparency, security, and verifiability in cryptocurrency networks.

The specific representation can vary based on the blockchain's architecture and consensus mechanism.

For example:
  - UTXO Model: In blockchains like Bitcoin, coins are represented as Unspent Transaction Outputs (UTXOs). Each UTXO can be thought of as a separate coin that is stored on the blockchain. Users' balances are derived from the total value of their UTXOs.
  - Account-based Model: In Ethereum, balances are stored in accounts. Each account has an address, and its balance is updated as transactions occur. The current state of all accounts and their balances is stored in the blockchain.

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

# Smart Contracts
A **smart contract** is a self-executing program that automates the actions required in a blockchain transaction.
It is represented by a program that contain agreements, or if-then conditions, that self-executing.
Since the process is entirely self-contained there does not need to be a 3rd party handling any
transactions between a buyer and seller, such as a lawyer.

# Layer 1 Network and Blockchain
A Layer 1 network is a network that acts as infrastructure for other applications, protocols, and networks to build on top of.
A Layer 1 blockchain, therefore, is hence the foundation of a blockchain network. It is responsible for:
  - Transaction processing
  L1 blockchains are responsible for all on-chain transactions, including executing and confirming them.
  - Consensus mechanisms
  - L1 blockchains use consensus models to process transactions, which vary by network and purpose.
  Infrastructure
  - L1 blockchains provide the infrastructure for smart contracts and decentralized applications.
  Native cryptocurrency
  - L1 blockchains use a native cryptocurrency to pay transaction fees and reward network security.
  Security

# Web3?
**Web3** is a term for a decentralized internet that gives users more control over their data and online activity.

It *uses* blockchain technology to create a secure, transparent, and tamper-proof environment for data storage and transactions.
  - That is, Blockchain focuses on enables transparent recording transactions, while Web3 allows interactions between users and applications in a more decentralised and democratic way.

Web3 applications, such as decentralized finance (DeFi), allow for financial services like lending, borrowing, and trading without the need for traditional banks.

It aims to shift from the centralized Web2 model, dominated by large corporations, to a more open and trustless ecosystem.

## Web3: Evolution of the Internet

- **Web1**: Static content, read-only.
- **Web2**: Interactive platforms dominated by major companies.
- **Web3**: Decentralized applications (dApps) and user ownership.

## Components of Web3

1. **Decentralized Applications (dApps)**: Applications running on blockchain networks.
2. **Smart Contracts**: Self-executing contracts automating agreements.
3. **Decentralized Finance (DeFi)**: Financial services without intermediaries.
4. **Decentralized Autonomous Organizations (DAOs)**: Community-governed entities.
5. **Non-Fungible Tokens (NFTs)**: Unique digital assets representing ownership.
6. **Cryptocurrencies**: Digital tokens for transactions and value storage.

## Use Cases

- **Decentralized Social Media**: User-controlled content and data.
- **DeFi**: Peer-to-peer financial services.
- **Marketplaces**: Direct commerce without intermediaries.
- **Digital Identity**: Self-sovereign identities for users.
- **Gaming**: Ownership of in-game assets.

# Decentralized Application: dApps

A **dApp** (decentralized application) is an application that runs on a decentralized network (like a blockchain). It typically consists of:
  - **Smart Contracts**: The backend logic of the application that runs on the blockchain.
  - **Frontend Interface**: A user interface that allows users to interact with the smart contracts (often a web or mobile app).
  - **Decentralization**: The application is designed to operate without a central authority, relying on the blockchain for data storage and execution.