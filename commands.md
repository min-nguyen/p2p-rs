─────────────────────────────────────────────
    # Available Commands #
─────────────────────────────────────────────
  *Load chain*:
└── Usage: `load`
┌── Description:
│     • Load a chain to the application from a (predefined) local file `blocks.json`.

  *Save chain*:
└── Usage: `save`
┌── Description:
│     • Save the current chain to a (predefined) local file `blocks.json`.

  *Reset blockchain*:
└── Usage: `reset`
┌── Description:
│     • Reset current chain to a single block.

  *Mine new block*:
└── Usage: `mine ?[data]`
┌── Description:
|     • If no arguments are provided:
|       -  mine a block from the first transaction in the pool (if any), adding it to the chain, and broadcasting it to other peers.
│     • If an an argument is provided:
|       -  mine a block with the given data, adding it to the chain, and broadcasting it to other peers.

  *Create new transaction*:
└── Usage: `txn [data]`
┌── Description:
│     • Create a (random) transaction with the amount set to the given data, adding it to the pool, and broadcasting it to other peers.

  *Request chain from peers*:
└── Usage: `req <"all" | [peer-id]>`
┌── Options:
│     • `"all"`      - Request chain from all peers
│     • `[peer-id]`  - Request chain from a specific peer

  *Show peers/chain/transaction pool*:
└── Usage: `show <"peers" | "chain" | "txns">`
┌── Options:
│     • `"peers"`   - Show a list of discovered and connected peers
│     • `"chain"`   - Show current chain
│     • `"txns"`    - Show transaction pool

  *Redial*:
└── Usage: `redial`
┌── Description:
│     • Redial all discovered peers.

  *Command menu*:
└── Usage: `help`
┌── Description:
│     • Prints this list of commands.
─────────────────────────────────────────────