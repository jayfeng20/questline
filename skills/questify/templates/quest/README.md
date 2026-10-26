# __QUEST__ — a runnable quest

<one paragraph: what the whole quest covers, the crate versions it pins, who it is for>

The quest is one Cargo workspace; each module under `modules/` is a member crate sharing its
dependencies and `target/`.

## Course map

| # | module | you will be able to | lessons | status |
|---|--------|---------------------|---------|--------|

## Start

```bash
cd modules/<module>
cargo run -- list                 # the syllabus, with your progress
cargo run -- 1                    # lesson 1
cargo run -- check 1              # test your answers for lesson 1 and save progress
```

Or run `questline` from the repository root and pick a module: `e` opens it in your IDE.
