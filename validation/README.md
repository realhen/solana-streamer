# Streamer fork review — 2026-09-25 UTC

Baseline: `0xfnzero/solana-streamer` revision `894de545c1ddd1f222174dc7b6c90b39e9c1ce40`, crate `solana-streamer-sdk` 3.0.5.

The facade is now pinned to `realhen/sol-parser-sdk` revision `5e685c5f86570ec26e668c77417b7b3d5967d1df`, version 0.7.6. It no longer depends on an unversioned sibling checkout or a floating 0.7.5-compatible parser. Cargo.lock records the tested resolution. Existing Orca/DLMM test fixtures were updated for the parser's current event schema.

Two new offline integration workflows replay the same public ten-transaction corpus and 21 account variants used by the parser fork's official Pump SDK oracle. They verify trade fields and slots through the actual streamer facade and account bridge. The account bridge intentionally normalizes Pump's zero quote mint to `So11111111111111111111111111111111111111111`; the parity test accounts for that documented representation difference.

Verification passed: 82 existing library checks, two new offline replay workflows, and seven existing gated mainnet replay workflows (Pump/PumpSwap upgrade tails, Meteora DAMM v2/DLMM, LaunchLab and instruction-account forwarding). No transaction was signed or submitted.

```sh
cargo +1.97.1 test --locked --lib --test fork_replay
RUN_MAINNET_TESTS=1 cargo +1.97.1 test --locked --test current_mainnet_transactions
```

The second command reads from `SOLANA_RPC_URL`. Public fixtures are committed under `validation/fixtures`; their oracle and exact npm pins are in the [parser fork review](https://github.com/realhen/sol-parser-sdk/blob/5e685c5f86570ec26e668c77417b7b3d5967d1df/validation/README.md).

This validates decoding and adaptation, not live LaserStream connection lifecycle, dynamic-filter delivery, reconnect gap recovery, reorg handling or transaction landing. The engine still owns its one-connection-per-deployment subscription lifecycle; this fork does not silently replace that transport. Use default `sdk-parse-borsh`; other parser/performance modes need separate validation before deployment.
