# NEXUS network promotion plan

## Network stages

1. Devnet — protocol development and deterministic unit/integration tests.
2. Testnet — multi-node network, validator rotation, persistence/restart, transaction propagation and consensus failure testing.
3. Mainnet candidate — frozen protocol version, frozen genesis, audited configuration, reproducible builds, backup/restore and operational runbooks.
4. Mainnet — production chain only after the candidate passes the full release gate.

## Mainnet release gate

- [ ] Consensus safety tests pass.
- [ ] Every validator ID is bound to a trusted Ed25519 public key.
- [ ] Every production node passes `Node::validate_authenticated_consensus_ready()` before networking/consensus starts.
- [ ] Consensus-enabled nodes reject unsigned consensus messages and direct `LatestBlock` acceptance.
- [ ] Consensus liveness tests pass with validator failures.
- [ ] P2P reconnect and message-size limits tested.
- [ ] Chain/state reconstruction validation passes after restart.
- [ ] Wallet keystore backup/restore tested.
- [ ] Genesis hash and chain ID are frozen.
- [ ] Maximum supply is frozen at 100,000,000 NEXUS.
- [ ] No unapproved minting path exists.
- [ ] Testnet has run through upgrade/restart/fork scenarios.
- [ ] Release artifacts are reproducible.
- [ ] Monitoring, backups and incident procedures are documented.
- [ ] External security review is completed before production funds are exposed.

## Important separation

NEXUS Messenger does not depend on this chain for ordinary messages. Chat remains free and off-chain. Blockchain integration is an optional wallet/value layer.

## Network IDs

- Devnet: 1000003
- Testnet: 1000002
- Mainnet: 1000001

The IDs are protocol constants in src/config.rs and must not be changed after genesis is frozen.

## Authenticated consensus startup gate

Before a production node is allowed to start its consensus/network service, configure its validator set, local node ID, local wallet signer, and the trusted public key for every validator. Call `validate_authenticated_consensus_ready()` and abort startup on any error. Do not accept public keys from incoming messages as a source of validator identity. The Node API now rejects direct `LatestBlock` messages when a validator set is configured and validates a proposed block against a cloned chain before voting.

This API check must be wired into the eventual executable/service startup path. The repository does not yet have a complete production daemon or a formal external security audit; these checks alone do not make mainnet production-ready.
