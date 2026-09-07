# Native test count correction

The first native suite was reported as 110 tests instead of 100. Later handoffs inherited the ten-test overcount.
This audit counts actual test groups from `cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked -- --list`.
It corrects totals, not execution results. The recorded test runs passed; no test was removed to produce these counts.

| Slice | Validated implementation PR | Native tests |
| --- | --- | --- |
| S01 | [75](https://github.com/mia-cx/ditherette/pull/75) | 100 |
| S02 | [88](https://github.com/mia-cx/ditherette/pull/88) | 100 |
| S03 | [89](https://github.com/mia-cx/ditherette/pull/89) | 114 |
| S07 | [91](https://github.com/mia-cx/ditherette/pull/91) | 124 |
| S08 | [92](https://github.com/mia-cx/ditherette/pull/92) | 124 |
| S09 | [94](https://github.com/mia-cx/ditherette/pull/94) | 124 |
| S10 | [97](https://github.com/mia-cx/ditherette/pull/97) | 158 |
| S11 | [93](https://github.com/mia-cx/ditherette/pull/93) | 126 |
| S12 | [95](https://github.com/mia-cx/ditherette/pull/95) | 143 |
| S13 | [98](https://github.com/mia-cx/ditherette/pull/98) | 168 |
| S14 | [99](https://github.com/mia-cx/ditherette/pull/99) | 159 |

Each total belongs to that slice's own dependency tree, not the eventual full join.
The PR descriptions now use these totals. This table supersedes older prose totals in ancestor handoff documents and comments.
When S17 joins those documents, correct their totals from this table without rewriting the validated implementation histories.
Future validation records derive totals from actual output groups, not sums copied from earlier prose.
