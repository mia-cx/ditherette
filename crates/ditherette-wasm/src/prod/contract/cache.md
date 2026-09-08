# Literal cache control baseline

`cache.rs` copies the frozen state model byte-for-byte. Relative contract imports use existing production contracts.
Read [the frozen allocation boundary](../../spec/contract/cache.md) before integrating real ownership.
The tiny modeled values and declared capacities do not enforce physical allocation limits.
S31 integrates preparation and scratch only. Image-stage caching remains S32 work.
