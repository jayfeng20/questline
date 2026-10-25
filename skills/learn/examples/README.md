# learn skill examples — a verified example quest

The quest layout `/learn` writes, with one module per lesson archetype (see
[`../references/examples.md`](../references/examples.md)). It lives inside the skill, not under
`quests/`, so the dashboard does not list it; `questline validate skills/learn/examples` checks it.

## Course map

| # | module | you will be able to | lessons | status |
|---|--------|---------------------|---------|--------|
| 1 | [std-and-tokio](modules/std-and-tokio/) | explain Vec growth, tokio workers and Arrow streaming with evidence, then rebuild HashMap and Future against std | 5 | **generated** |
| 2 | [lance-dataset](modules/lance-dataset/) | write, open, scan, take and time-travel a Lance dataset, with the bytes each read costs | 1 | **generated** |

`lance-dataset` pins its own Arrow 58 (the major lance 13 links against) instead of the
workspace's 59: the one case where a module does not inherit a shared pin.

## Start

```bash
cd modules/std-and-tokio
cargo run -- list
cargo run -- 1
cargo run -- check 1
```
