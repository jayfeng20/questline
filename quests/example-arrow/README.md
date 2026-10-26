# example-arrow — the sample quest

A complete, registered quest with one module, so a fresh clone of questline has something to
show. It is a quest like any other (see [`docs/contract.md`](../../docs/contract.md)): a Cargo
workspace with a `quest.toml`, and one member crate per module under `modules/`.

| module | you will be able to | lessons |
|--------|---------------------|---------|
| [arrays-and-recordbatch](modules/arrays-and-recordbatch/) | build and downcast primitive, string and nested arrays, choose an encoding by measured memory, assemble Schema + RecordBatch + RecordBatchReader | 5 |

```bash
cd modules/arrays-and-recordbatch
cargo run -- list        # the syllabus, with your progress
cargo run -- 1           # lesson 1
cargo run -- check 1     # test your answers for lesson 1 and save progress
```

`example = true` in `quest.toml` keeps `/questify` autosort from adding modules here: generate
your own quests with the skill instead.
