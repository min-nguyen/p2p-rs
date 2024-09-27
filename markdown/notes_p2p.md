# P2P

A P2P network is one in which interconnected nodes ("peers") share resources amongst each other without the use of a centralized administrative system.

# Network Behaviours
In a P2P network, a network behavior defines how a peer interacts with the network, including how it sends and receives messages, handles connections, and manages protocols (like Pub/Sub, direct messaging, etc.).
In Rust's libp2p, a behavior is often implemented as a struct that encapsulates different protocols, events, and handlers.

For peers in a P2P network to coexist and communicate effectively, they need to have the following components in common:
 - Communication Protocol
    - Like FloodSub
 - Peer Discovery Protocol
    - Like mDNS
 - Other things:
  - Message formats
  - Connection management strategies
  - Message handling and routing mechanisms
  - Security and authentication measures
  - Common event handling capabilities


## Communication protocol: FloodSub

FloodSub is a publish-subscribe protocol:
1. Publishers send messages to all peers they are directly connected to, without any filtering.
2. Subscribers receive messages by subscribing to specific topics.
3. When a message is published, it is flooded to all peers in the network, and
  each peer forwards the message to their connected peers until the message reaches all interested nodes.

### Topics
A topic is a named channel that we can subscribe to and send messages on, in order to only listen to a subset of the traffic on a pub/sub network.