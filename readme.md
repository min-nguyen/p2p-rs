
/*************************************************************************************************************

                    ========================================================================>      SWARM
                     |                                                                               |
                     |                                            LOCAL_IO                           |
                     |                                                ↑                          req/response
                     |                                             fn call                           |
                     |                                                ↑                              ↓
 STDIN ==>         PEER    =============== fn call =======> LOCAL_NETWORKBEHAVIOUR <== event <==  P2P NETWORK
           { LOCAL_RECEIVER } <========== response ===========  { LOCAL_SENDER }

**************************************************************************************************************/


```sh
RUST_LOG=info cargo run
```