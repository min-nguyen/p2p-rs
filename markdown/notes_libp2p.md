# LibP2P

A P2P network is one in which interconnected nodes ("peers") share resources amongst each other without the use of a centralized administrative system.

# Network Behaviours + Swarms

NetworkBehaviour and Swarm are part of Rust’s libp2p library for building peer-to-peer (P2P) networking applications.

1. NetworkBehaviour: specifies how a node should behave (e.g., handling requests, responses, discovery protocols).

   - A NetworkBehaviour is a high-level abstraction that defines how a peer (node) behaves on the network.

      Defines the Peer’s Network Logic:
         Specifies how the peer should react to network events (e.g., messages, connections).
         Encapsulates the application’s protocol-specific behaviours (e.g., handling PubSub, file sharing, or custom messaging).
      Handles Inbound and Outbound Requests:
         Implements logic for sending, receiving, and responding to messages.
         Can initiate actions such as peer discovery or data retrieval.
      Combines Multiple Protocols:
         Allows composing and combining multiple protocols into a single NetworkBehaviour.
         Manages different behaviours like routing, broadcasting, or peer discovery under a unified interface.
      Generates Events for the Swarm:
         Dispatches events that the Swarm will act upon (e.g., initiate a connection, send data).
         Receives network-level events from the Swarm and decides how to respond.
      Acts as the Control Layer:
         Provides a structured way to define what actions to take in different network situations.
         Lets you customize peer interactions without dealing directly with low-level networking details.

   Essentially, it defines the communication protocols and the handling of different behaviours (e.g., discovery, messaging) that a node is expected to perform.

2. Swarm: a container and driver for the network behaviour.

   - The Swarm is a lower-level construct that manages the networking aspects of a libp2p application.
   - It wraps the NetworkBehaviour and provides the infrastructure to drive the execution of the defined behaviours.

      Manages Peer Connections:
         Establishes and accepts network connections.
         Handles peer connection lifecycle events (e.g., connections, disconnections).

      Drives NetworkBehaviour Execution:
         Polls the defined NetworkBehaviour for actions to execute.
         Routes incoming and outgoing requests/responses as defined by your NetworkBehaviour.

      Coordinates Protocol Interactions:
         Implements and manages multiple communication protocols (e.g., Kademlia, PubSub).
         Ensures correct protocol-level messaging between peers.

      Processes Network Events:
         Captures and handles events like peer discovery, errors, and custom protocol interactions.
         Allows you to react programmatically to these events (e.g., log connections, modify state).

      Serves as the Main Event Loop:
         Central point for driving asynchronous operations in your P2P application.
         Continuously updates and manages the state of your network.

   Essentially, it is the "engine" that powers all network-related tasks, enabling the NetworkBehaviour to function as intended, making it crucial for coordinating peer activities in your P2P app.

In short, a Peer owns:
   1. A NetworkBehaviour which defines what logic should happen when handling events and interactions.
   2. A Swarm which provides the event loop to process all network events and trigger the user-defined NetworkBehaviour.
   3. At run-time, all communication (described in code) between a Peer and its NetworkBehaviour is actually driven by the Swarm as an intermediary.

Without a Swarm, a NetworkBehaviour is just a static configuration that cannot perform any actions or respond to network events, as there is no runtime component to drive it. If you try to run a peer with only a NetworkBehaviour and no Swarm:
   - The peer won't establish or receive any connections.
   - It won't participate in peer discovery, so other peers will not be aware of its existence.
   - It cannot communicate or handle any P2P protocol messages.

## LibP2P: NetworkBehaviour

A `NetworkBehavior` is a struct that configures different protocols, events, and handlers.

For peers in a P2P network to coexist, they need to have some high-level components in common:
 - Communication Protocol, Like FloodSub
 - Peer Discovery, Like mDNS
 - Other things:
  - Message formats
  - Connection management strategies
  - Message handling and routing mechanisms
  - Security and authentication measures
  - Common event handling capabilities

### Communication protocol: FloodSub

FloodSub is a publish-subscribe protocol:
1. Publishers send messages to all peers they are directly connected to, without any filtering.
2. Subscribers receive messages by subscribing to specific topics.
3. When a message is published, it is flooded to all peers in the network, and
  each peer forwards the message to their connected peers until the message reaches all interested nodes.

#### Topics
A topic is a named channel that we can subscribe to and send messages on, in order to only listen to a subset of the traffic on a pub/sub network.


## LibP2P: Swarm

A `Swarm` is an object that wraps around a NetworkBehaviour.

- `self.swarm.select_next_some().await`: returns the next event generated by the Swarm, which can then be processed in the main event loop.