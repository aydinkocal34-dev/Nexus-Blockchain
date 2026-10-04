# NEXUS P2P inbound routing

Application messages received from a connected peer are delivered to the node through a bounded inbound event channel. Ping/Pong remains handled at the transport layer. Node-level responses are sent back to the originating peer.

Consensus proposal and vote messages are transported to the node, while consensus validation and commit rules remain governed by the NEXUS protocol design.
