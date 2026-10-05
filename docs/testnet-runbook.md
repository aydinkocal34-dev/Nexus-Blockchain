# NEXUS Testnet Runbook

## Topology

The integration suite uses four validator nodes with deterministic node IDs and ephemeral localhost ports. The same topology is the baseline for a real Testnet deployment.

## Required checks

1. Start four nodes with the same protocol version and Testnet chain ID.
2. Verify every node has a unique validator identity.
3. Establish persistent P2P connections.
4. Propagate a transaction and verify mempool acceptance.
5. Propose a block and collect validator votes.
6. Require quorum before commit.
7. Restart a node and reload persistent storage.
8. Synchronize missing blocks from a peer.
9. Repeat with one validator offline.
10. Verify all nodes converge on the same height and state.

The automated suite now covers the four-node topology and four-node consensus commit path. Production Testnet still requires real process-level deployment and failure testing before Mainnet Candidate.
