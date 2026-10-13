# Authenticated consensus messages

NEXUS supports Ed25519-authenticated consensus messages through
`SignedProposal` and `SignedVote`. The authenticated path is opt-in at the
node configuration layer so existing development/testnet fixtures can continue
to exercise the legacy protocol while deployments migrate.

## Configure each validator node

1. Configure the validator set.
2. Set the node's validator identity.
3. Load that validator's wallet from its encrypted keystore and pass it to
   `Node::configure_consensus_signer`.
4. Call `Node::configure_validator_public_key` for every validator ID in the
   set, using the public key registered for that validator.

Once a signer or validator public key is configured, the node enters
authenticated-consensus mode. In that mode, legacy unsigned `ProposeBlock`
and `Vote` messages are rejected. Signed proposals bind the height, block
hash, and proposer ID; signed votes bind the height, block hash, voter ID, and
commit/reject decision. Verification also checks that the public key matches
the configured key for the claimed validator identity.

## Safety notes

- Never accept a public key supplied only by the incoming message as the
  validator's identity. Bind validator IDs to keys from trusted chain
  configuration.
- Configure every validator key before enabling production consensus. Missing
  keys intentionally cause signed messages to be rejected.
- The legacy unsigned path remains available when authenticated mode is not
  enabled. Production startup must therefore explicitly configure the signer
  and full validator-key registry; this is not yet enforced by the mainnet
  release gate.
- This change does not by itself provide encrypted transport, authenticated
  peer handshakes, consensus rounds/timeouts, fork-choice/reorganization, or an
  external security audit. Do not treat it as mainnet-ready cryptography.

The authenticated path is covered by `tests/consensus_auth_tests.rs` and
`tests/consensus_auth_node_tests.rs`.
