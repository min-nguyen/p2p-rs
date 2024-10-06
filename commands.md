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
└── Usage: `mine [data]`
┌── Description:
│     • Mine and add a new block to the chain, broadcasting this to other peers.

  *Request chain from peers*:
└── Usage: `req <"all" | [peer-id]>`
┌── Options:
│     • `"all"`      - Request chain from all peers
│     • `[peer-id]`  - Request chain from a specific peer

  *Show chain or peers*:
└── Usage: `show <"peers" | "chain">`
┌── Options:
│     • `"peers"`   - Show a list of discovered and connected peers
│     • `"chain"`   - Show current chain

  *Redial*:
└── Usage: `redial`
┌── Description:
│     • Redial all discovered peers.

  *Command menu*:
└── Usage: `help`
┌── Description:
│     • Prints this list of commands.
─────────────────────────────────────────────