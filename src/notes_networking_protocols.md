
# Communication Protocols (in the TCP/IP Model) in Backend Development: A Concise Guide

Backend development often involves multiple communication protocols to handle client-server interactions, data transfers, and system integration.

## TCP/IP Model (The Internet Protocol Suite)
TCP/IP is known as the Internet Protocol Suite, and provides practical 4-layer model that addresses specific communication challenges and relies on standardized protocols. Different communication protocols belong to different layers in the TCP/IP model.

The OSI model is an alternative framework, and provides a theoretical 7-layer model that emphasizes clear separation of functions.

### 1. **Application Layer**
The application layer of TCP/IP model provides applications the ability to access to services of the other layers, and defines the protocols that applications use to exchange data. Most widely-known application layer protocols include HTTP, WebSocket, FTP, SMTP, Telnet, DNS, SNMP and Routing Information Protocol (RIP).
### 2. **Transport Layer**
The transport layer, also known as the host-to-host transport layer, is responsible for providing the application layer with session and datagram communication services. The core protocols of this layer are TCP and UDP. TCP provides a one-to-one, connection-oriented, reliable communications service. It is responsible for sequencing and acknowledgment of packets sent, and recovery of packets lost in transmission. UDP provides one-to-one or one-to-many, connectionless, unreliaable communications service. UDP is used typically when the amount of data to be transferred is small (such as that data would fit into a single packet).
### 3. **Internet Layer**
The Internet layer is responsible for host addressing, packaging, and routing functions. The core protocols of the Internet protocol layer are IP, Address Resolution Protocol (ARP), Internet Control Message Protocol (ICMP) and Internet Group Management Protocol (IGMP). The IP is a routable protocol responsible for IP addressing, routing, and the fragmentation and reassembly of packets.
### 4. **Network Access Layer**
Network access layer (or link layer) is responsible for placing the TCP/IP packets on the network medium and receiving TCP/IP packets off the network medium. TCP/IP is designed to be independent of the network access method, frame format, and medium. In other words, it is independent from any specific network technology. In this way, TCP/IP can be used to connect different network types, such as Ethernet, Token Ring, X.25, Frame Relay, and Asynchronous Transfer Mode (ATM).

## TCP/IP Communication Protocols
Here’s an overview of the essential TCP/IP communication protocols:

Layer | Communication Protocols

Application Layer : HTTP/HTTPS, WebSocket
Transport Layer   : TCP, UDP
Internet Layer    : IP

---

### 1. **HTTP/HTTPS** (Application Layer)
**HTTP** is a protocol used for transmitting data over the web. It facilitates the request-response model between clients (like web browsers)  and servers, letting clients send requests (to fetch or send data), and servers respond with the requested data, (web pages, API responses, or resources). **HTTPS** is the secure version that encrypts data using SSL/TLS.

**HTTP** operates at the **application layer** and is the most basic backbone of REST APIs and and data exchange on the internet.

- **Usage**: Enables clients to access resources from a server via a request-response model.
- **Examples of Usage**:
    1. **Web Browsing**:
       - **Example**: Accessing a website like example.com
       - **Description**: HTTP is used to request web pages and resources from a server, allowing users to view and interact with websites.
    2. **APIs**:
       - **Example**: Fetching data from a RESTful API endpoint (e.g., `https://api.example.com/users`)
       - **Description**: HTTP methods like GET and POST are used to interact with web APIs, enabling applications to retrieve or send data to a server.
    3. **Form Submission**:
       - **Example**: Submitting a contact form on a website
       - **Description**: HTTP POST is used to send form data to a server for processing, such as submitting user information or feedback.
    4. **File Downloads**:
       - **Example**: Downloading a file from a website
       - **Description**: HTTP GET requests are used to retrieve files from a server, allowing users to download documents, images, or software.
    5. **Web Services**:
       - **Example**: Accessing a web-based service like Google Maps
       - **Description**: HTTP is used to interact with web services, enabling functionalities like location-based services and interactive maps.
- **Properties**:
    1. **Stateless**: Each request from a client to a server is independent. The server does not retain any information about previous requests.
    2. **Request-Response Model**:
      - HTTP follows a request-response model where the client sends a request to the server, and the server responds with the requested resource or an error message.
    3. **Flexible and Extensible**:
      - HTTP headers provide metadata about the request or response, and additional fields can be added to extend functionality. For example, custom headers can be used for authentication or content negotiation.
    4. **Methods**:
      - HTTP defines several methods (or verbs) to perform different operations, including:
        - **GET**: Retrieve data from the server.
        - **POST**: Send data to the server (e.g., form submissions).
        - **PUT**: Update or create resources on the server.
        - **DELETE**: Remove resources from the server.
    5. **Status Codes**:
      - HTTP responses include status codes indicating the outcome of the request. For example:
        - **200 OK**: The request was successful.
        - **404 Not Found**: The requested resource could not be found.
        - **500 Internal Server Error**: The server encountered an error processing the request.
    6. **Content Negotiation**:
      - HTTP supports content negotiation, allowing clients and servers to agree on the format of the data being exchanged (e.g., HTML, JSON, XML).

#### Summary:
HTTP is a crucial protocol for web communication, enabling the request and delivery of web resources. It supports a wide range of applications, from browsing websites and submitting forms to interacting with APIs and downloading files. Its stateless nature and flexible methods make it foundational to web interactions and services.

---

### 2. **WebSocket** (Application Layer)
**WebSocket**  is a protocol that enables full-duplex, real-time communication between a client (e.g., a web browser) and a server over a single, long-lived TCP connection. It is designed to provide a more interactive and efficient way of communication compared to traditional HTTP.

**WebSocket** operates at the **application layer** and builds on top of TCP, allowing for bidirectional communication that is ideal for low-latency real-time applications.

- **Usage**: Enables continuous real-time communication between a client and server (chat apps, live updates, gaming).
- **Properties**:
    1. **Full-Duplex Communication**:
      - WebSocket allows simultaneous two-way communication between client and server over a single connection. This means both parties can send and receive messages at any time.
    2. **Persistent Connection**:
      - Once a WebSocket connection is established, it remains open for the duration of the session, enabling continuous communication without the need for repeated handshakes or reconnecting.
    3. **Low Latency**:
      - WebSocket reduces the latency associated with opening new connections or using polling techniques, providing near-instantaneous data transfer which is crucial for real-time applications.
    4. **Upgrade from HTTP**:
      - WebSocket connections start with an HTTP handshake to establish the initial connection, but then switch to the WebSocket protocol, leveraging the same TCP connection for ongoing communication.
    5. **Message Framing**:
      - WebSocket messages are sent in frames, allowing for efficient transmission of data. Frames can be of various sizes and can include control frames, data frames, and ping/pong frames.
    6. **Low Overhead**:
      - Compared to HTTP, WebSocket has lower overhead since it avoids the need for headers and metadata in each message, improving performance and efficiency in data transfer.
- **Examples of Usage**:
    1. **Real-Time Chat Applications**:
       - **Example**: Slack, WhatsApp Web
       - **Description**: WebSocket enables instant messaging between users, allowing messages to be delivered and received in real-time without delays or constant polling.
    2. **Live Notifications**:
       - **Example**: Twitter, Facebook Live Updates
       - **Description**: WebSocket provides real-time notifications and updates to users about new posts, likes, or comments, ensuring users are always informed of the latest activity.
    3. **Online Gaming**:
       - **Example**: Fortnite, Agar.io
       - **Description**: WebSocket allows real-time synchronization between players, providing smooth and responsive game experiences by continuously updating game states and actions.
    4. **Collaborative Tools**:
       - **Example**: Google Docs, Trello
       - **Description**: WebSocket facilitates real-time collaboration by enabling multiple users to simultaneously edit documents or boards, with changes reflected instantly across all participants.
    5. **Financial Trading Platforms**:
       - **Example**: Stock trading apps
       - **Description**: WebSocket enables real-time updates on stock prices, trades, and market data, allowing traders to make informed decisions with the latest information.

#### Summary:
WebSocket provides a full-duplex, persistent connection over TCP, allowing for real-time, low-latency communication between clients and servers. It is especially useful for applications requiring continuous, bidirectional data exchange, such as chat applications, live feeds, online games, and collaborative tools.


---

### 3. **TCP/IP (Transmission Control Protocol/Internet Protocol)** (Transport Layer/Internet Layer)
**TCP/IP** is the fundamental protocol suite for networking, forming the basis for the Internet and most other networks. It consists of two main protocols:
- **TCP (Transmission Control Protocol)**: Ensures reliable, ordered, and error-checked delivery of data packets.
- **IP (Internet Protocol)**: Handles addressing and routing of packets across networks.

**TCP/IP**, as a protocol suite, operates at the **transport layer**
- **TCP** specifically operates at the **transport layer**
- **IP** specifically operates at the **internet layer**.
Together, they form a foundational protocol for many **application layer** protocols (such as HTTP and WebSocket),
providing reliable, ordered, and error-checked data transmission for application layer protocols to function properly.

- **Usage**: Ensures reliable transmission of data and underlies most internet traffic.
- **Properties**:
    1. **Connection-Oriented**:
      - TCP establishes a reliable connection between the client and server before data transfer begins (via a three-way handshake). This ensures both parties are ready to communicate.
    2. **Reliable Data Transfer**:
      - TCP guarantees that data sent from the source will reach its destination without errors, loss, or duplication. It uses acknowledgments (ACKs) to confirm receipt of data.
    3. **Ordered Delivery**:
      - TCP ensures that packets (data segments) arrive in the correct order, even if they are received out of sequence by the network.
    4. **Error Detection and Correction**:
      - TCP uses checksums to detect errors in transmitted data and ensures corrupted packets are re-sent. It provides mechanisms to correct errors automatically.
    5. **Flow Control**:
      - TCP uses flow control to manage the rate of data transmission between sender and receiver, ensuring that a sender does not overwhelm the receiver with too much data at once (using a sliding window mechanism).
    6. **Congestion Control**:
      - TCP monitors network congestion and adjusts the rate of data transmission to prevent packet loss and ensure smooth communication under network load (using algorithms like slow start and congestion avoidance).
    7. **Full-Duplex Communication**:
      - TCP allows both the sender and receiver to send and receive data simultaneously, supporting two-way communication.
    8. **Stream-Oriented**:
      - TCP treats data as a continuous stream of bytes rather than individual packets, making it easier for applications to read/write data without worrying about packet boundaries.
- **Examples of Usage**:
    1. **Web Browsing (HTTP/HTTPS)**:
       - **Example**: Visiting a website like example.com
       - **Description**: TCP ensures reliable and ordered delivery of web pages and resources from a server to the browser, making sure the content is displayed correctly.
    2. **File Transfer (FTP/SFTP)**:
       - **Example**: Uploading or downloading files from a server using FileZilla
       - **Description**: TCP guarantees that files are transferred completely and without corruption, ensuring that large files arrive intact.
    3. **Email Communication (SMTP/IMAP/POP3)**:
       - **Example**: Sending or receiving emails via Outlook or Gmail
       - **Description**: TCP ensures that email messages are transmitted reliably between mail servers and email clients, preventing message loss or duplication.
    4. **Remote Access (SSH)**:
       - **Example**: Connecting to a remote server using an SSH client like PuTTY
       - **Description**: TCP provides a secure, reliable connection for remote command-line access and administration of servers.
    5. **Database Communication**:
       - **Example**: Accessing a MySQL database from an application
       - **Description**: TCP ensures reliable communication between database clients and servers, allowing for consistent data retrieval and updates.
    6. **File Sharing (SMB/CIFS)**:
       - **Example**: Sharing files over a network using Windows file sharing
       - **Description**: TCP allows for reliable and efficient file sharing between computers on a network, ensuring that files are transferred accurately.
    7. **Streaming Services**:
       - **Example**: Streaming a video on Netflix
       - **Description**: TCP ensures that video data is transmitted reliably and in the correct order, providing a smooth viewing experience with minimal interruptions.

    ```
            Client                               Server
            ------                               ------
    1.    Socket()                             Socket()
          (Create socket)                      (Create socket)

    2.    Connect()  ---> [SYN] -------------> Bind()
          (Request connection)                 (Bind to a port)

                                                Listen()
                                              (Ready to accept connections)

    3.             <--- [SYN, ACK] <----------
          (Server acknowledges)

    4.    [ACK]      ---> [ACK] ------------->
          (Connection established)

    ---------------- CONNECTION ESTABLISHED ----------------

    5.    Send()     ---> [Data] ------------>
          (Send data)                         Recv()
                                              (Receive data)

    6.             <--- [Data] ---------------
          Recv()                             Send()
          (Receive data)                      (Send response)

    7.    Close()    ---> [FIN] ------------->
          (Client closes connection)

                    <--- [FIN, ACK] ----------
                                              Close()
                                              (Server closes connection)

    ---------------- CONNECTION TERMINATED ----------------
    ```

#### Summary:
TCP provides reliable, ordered, and error-checked data delivery, making it suitable for applications that require accuracy and consistency, such as web browsing, email, file transfers, and secure communications.
---

# Application Programming Inteface (API) Architectures

## 1. **REST APIs**.
A RESTful API is an architectural style for an application programming interface that uses HTTP requests to access and use data.- **Usage**: APIs for CRUD operations on resources, typically exposed via URLs.
- **Stateless**: Each request contains all the information the server needs.
- **JSON/XML**: Data is often transferred in JSON or XML format.
- **Idempotent Methods**: Operations like `GET` or `DELETE` can be safely repeated without causing side effects.

**Why it matters**: REST APIs are widely used for web services due to simplicity, scalability, and compatibility with HTTP.
