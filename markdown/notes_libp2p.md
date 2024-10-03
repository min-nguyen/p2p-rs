# LibP2P

A P2P network is one in which interconnected nodes ("peers") share resources amongst each other without the use of a centralized administrative system.

# Transport

The transport layer of the networking stack facilitates the end-to-end communication of data between different devices.
A transport refers to the mechanisms and protocols that operate primarily at the transport layer.
A transport is defined in terms of two core operations, listening and dialing.

(One of libp2p’s core requirements is to be transport agnostic.
   - the decision of what transport protocol to use is up to the developer,
   - an application can support many different transports at the same time)

## Transport: Listening & Dialing

#### Transport: Listening
This means that you can accept incoming connections from other peers, which you can accept to establish a full connection for exchanging messages.
- Listening is done using whatever facility is provided by the transport implementation. For example, a TCP transport on a unix platform could use the bind and listen system calls to have the operating system route traffic on a given TCP port to the application.

#### Transport: Dialing
Dialing is the process of opening an outgoing connection to a listening peer, which they can accept to establish a full connection.
- Like listening, the specifics are determined by the implementation, but every transport in a libp2p implementation will share the same programmatic interface.

#### Transport: Implicit vs Explicit Listening and Dialing
In many p2p implementations, especially libp2p, listening and dialing is often handled by the framework.
In most cases, P2P frameworks handle listening automatically, but explicit listening may be needed when:

Explicit manually listening can be necessary if:
   - You have custom listening requirements.
   - You need specific transport configurations.
   - You want to listen on multiple addresses or ports.
Explicit manually dialing can be necessary if:
   - You know the peer's ID and address and want to establish a direct connection.
   - You need to connect to peers discovered through a discovery mechanism.
   - You want to maintain or re-establish connections after disconnections or network changes.

# NetworkBehaviours + Swarms

NetworkBehaviour and Swarm are part of Rust’s libp2p library for building peer-to-peer (P2P) networking applications.

## NetworkBehaviour

NetworkBehaviour: specifies how a node should behave (e.g., handling requests, responses, discovery protocols).

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

For peers in a P2P network to coexist, they need to agree on some protocols to connect to each other, e.g:
 - Communication Protocol, Like FloodSub
 - Peer Discovery, Like mDNS
 - Other things:
  - Message formats
  - Connection management strategies
  - Message handling and routing mechanisms
  - Security and authentication measures
  - Common event handling capabilities
See [https://docs.libp2p.io/concepts/fundamentals/protocols/].

(In libp2p, a `NetworkBehavior` is a struct that configures different protocols, events, and handlers.)

## Swarm

Swarm: a container and driver for the network behaviour.

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

(In libp2p, `Swarm` is an object that wraps around a NetworkBehaviour. `self.swarm.select_next_some().await` returns the next event generated by the Swarm, which can then be processed in the main event loop.)

# Network Message Communication Protocols

## Communication Protocol: PubSub

A topic is a named channel that we can subscribe to and send messages on, in order to only listen to a subset of the traffic on a pub/sub network.

### PubSub: FloodSub

FloodSub is a publish-subscribe protocol:
1. **Publishers** broadcast messages to all directly connected peers whenever they choose to plsublish, without any filtering.
2. **Subscribers** receive messages by subscribing to specific topics.
3. **Message Propagation**: Each published message is flooded through the network as every peer forwards it to their connected peers until it reaches all subscribed nodes.

### PubSub: GossipSub

Gossipsub is an optimized publish-subscribe protocol that combines *flooding* and *gossiping* for efficient message dissemination:

**Message IDs**:
   - Message IDs are used to uniquely identify each message published in the Gossipsub network.
   - Peers will implicitly collect the heartbeats of other peers, and these heartbeats contain message IDs of published messages that those peers have seen.
   - This ensures that identical messages (with the same content) will have the same message ID, facilitating deduplication.
This allows peers to recognize and differentiate between different messages, even if they have similar content. Gossipsub prevents two messages with the same id from being sent. How the message id is generated can be customly done, allowing e.g. messages with simply the same content to only be sent once.

1. **Message Dissmenination**:
   1. Each topic has its own mesh.
   2. Peers **publish messages** only to peers within the same mesh.
   3. Publshing and subscribing works as usual.

   - Publishing: an explicit sharing of data, such as notifications about new blocks in a blockchain, user messages, or status updates.

2. **Gossip Dissemination**:
   1. Peers store message ids they have seen and their own subscriptions in a heartbeat data structure.
   2. Peers periodically **gossip heart beat messages (not full messages)** to a set of non-mesh peers.
   3. Those peers will then store other peers heartbeats in a temporary buffer.
      This information is used automatically by the gossipsub framework internally, (but can also be used explicitly by the programmer).

   Gossipsub Heartbeat Message {
    "message_ids": [ "msgid1", "msgid2", "msgid3", ... ],

    "subscriptions": [ "topic1", "topic2" ],

    "control_messages": {
        "graft": [ { "peer_id": "peer1", "topic": "topic1" }, ... ],
        "prune": [ { "peer_id": "peer2", "topic": "topic1" }, ... ],
        "ihave": [ { "peer_id": "peer3", "message_ids": ["msgid4", "msgid5"] }, ... ]
    }
}
   (In libp2p's `Gossipsub` implementation, you do not need to handle heartbeat messages explicitly. The Gossipsub protocol automatically manages the heartbeat messages for you as part of its internal operation. yYou can configure aspects of the Gossipsub protocol, such as the frequency of heartbeats and how many message IDs are included in each heartbeat. When you implement a Gossipsub protocol in your libp2p node, you typically focus on publishing and receiving messages. )

**Difference with Floodsub: Message dissemination**
Floodsub: doesn't have topic-specific meshes.
  1.  Uses a flooding mechanism for message dissemination.
  2.  When a peer receives a message, it forwards it to all its directly connected peers regardless of topic subscriptions or. This means that every peer floods the message to every other connected peer, leading to high redundancy and potential message duplication.
  3. As a result, Floodsub doesn’t have topic-specific meshes. There is no distinct mesh network for each topic; it just relies on the existing peer connections to flood messages.

