─────────────────────────────────────────────
    # Available Commands #
─────────────────────────────────────────────

📤 *Request data from peers*:
└── Usage: `req <"all" | [peer-id]>`
┌── Options:
│     • `"all"`      - Request last block from all peers
│     • `[peer-id]`  - Request last block from a specific peer

🔍 *Print a list*:
└── Usage: `ls <"peers" | "blocks">`
┌── Options:
│     • `"peers"`    - Show a list of connected remote peers
│     • `"blocks"`   - Show blocks stored in the local .json file

📝 *Write new data*:
└── Usage: `mk [data]`
┌── Description:
│     • Mine and write a new block to the local .json file.

  *Refresh data*:
└── Usage: `fresh`
┌── Description:
│     • Delete current blocks and write a new genesis block to the local .json file.

  *Redial*:
└── Usage: `redial`
┌── Description:
│     • Redial all discovered peers.
─────────────────────────────────────────────