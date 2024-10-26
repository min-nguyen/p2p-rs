─────────────────────────────────────────────
    # Available Commands #
─────────────────────────────────────────────
  *Load chain*:
└── Usage: `load ?[file_name]`
┌── Description:
│     • Load a chain to the application from a specified file name, defaulting to the file name `blocks.json`.

  *Save chain*:
└── Usage: `save ?[file_name]`
┌── Description:
│     • Save the main chain to a specified file name, defaulting to the file name `blocks.json`.

  *Reset blockchain*:
└── Usage: `reset`
┌── Description:
│     • Reset main chain to a single genesis block and delete existing forks.

  *Create new transaction*:
└── Usage: `txn [data]`
┌── Description:
│     • Create a (random) transaction with the amount set to the given data, adding it to the pool, and broadcasting it to other peers.

  *Mine new block*:
└── Usage: `mine ?[data]`
┌── Description:
|     • If no arguments are provided:
|       -  mine a block containing the first transaction in the pool (if any), adding it to the chain, and broadcasting it to other peers.
│     • If an argument is provided:
|       -  mine a block containing the given data, adding it to the chain, and broadcasting it to other peers.

  *Request chain from peers*:
└── Usage: `req <"all" | [peer-id]>`
┌── Options:
│     • `"all"`      - Request chain from all peers and synchronise to the most up-to-date chain
│     • `[peer-id]`  - Request chain from a specific peer and synchronise to the most up-to-date chain

  *Show peers/chain/forks/transaction pool*:
└── Usage: `show <"peers" | "chain" | "forks" | "pool" >`
┌── Options:
│     • `"peers"`   - Show list of discovered and connected peers
│     • `"chain"`   - Show main chain
│     • `"forks"`   - Show current forks from the main chain
│     • `"pool"`    - Show transaction pool

  *Redial*:
└── Usage: `redial`
┌── Description:
│     • Redial all discovered peers.

  *Command menu*:
└── Usage: `help`
┌── Description:
│     • Prints this list of commands.
─────────────────────────────────────────────