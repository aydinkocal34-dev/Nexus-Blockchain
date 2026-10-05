# NEXUS network promotion plan

## Network stages

1. Devnet — protocol development and deterministic unit/integration tests.
2. Testnet — multi-node network, validator rotation, persistence/restart, transaction propagation and consensus failure testing.
3. Mainnet candidate — frozen protocol version, frozen genesis, audited configuration, reproducible builds, backup/restore and operational runbooks.
4. Mainnet — production chain only after the candidate passes the full release gate.

## Mainnet release gate

- [ ] Consensus safety tests pass.
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
