
/*************************************************************************************************************

                     -------------------------------------------> SWARM ---------------------------->
                     ↑                                                                               |
                     |                                                                               |
             request/response                                                                   request/response
                     |                                                                               |
                     |                                                                               ↓
 STDIN ==>         PEER                                       LOCAL_NETWORKBEHAVIOUR <== event <--- P2P NETWORK
           { LOCAL_RECEIVER } <========== request ===========  { LOCAL_SENDER }
                    ↑
                    ↓
                LOCAL_IO
**************************************************************************************************************/

- peer:
  -
- local_network: receives messages from remote peers

```sh
RUST_LOG=info cargo run
```