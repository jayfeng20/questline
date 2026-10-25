# Scoping a broad topic

A topic is **too broad** when an honest module would need more than ~8 lessons or more than
one crate family ("tokio", "lance", "rust async", "arrow", "databases in rust"). Then do not
generate anything yet: show a table of contents and let the user pick.

## Format (use exactly this shape)

```
## <Topic> — table of contents

<one sentence: what the full map covers, pinned crate versions you will target>

| #  | module                     | you will be able to                              | lessons | needs                |
|----|----------------------------|--------------------------------------------------|---------|----------------------|
| 1  | <name>                     | <outcome, verb-first>                            | 3       | —                    |
| 2  | <name>                     | <outcome>                                        | 4       | module 1             |
| 3  | <name>                     | <outcome>                                        | 3       | dataset: NYC taxi (1.3 MB) |
...

Suggested paths:
- **<goal A>** (e.g. "use it in a service"): 1 → 2 → 5
- **<goal B>** (e.g. "contribute to it"): 1 → 3 → 6 → 7

Which modules should I generate? (numbers, a path, or "all")
```

Rules:
- 5–10 modules, ordered by dependency; each module is one focused course of 3–6 lessons.
- "you will be able to" is a concrete capability ("pick worker counts for CPU vs IO work and
  prove it with a measurement"), never "understand X".
- Put prerequisites and fixture downloads in **needs** so the user sees cost up front.
- The table of contents *is* the quest's course map: a broad topic is one quest, and each row
  is one module of it. After the user answers, place the quest (`references/placement.md`:
  an existing quest for the subject, or a new one), generate one module per chosen row under
  `modules/<module>/` (the row's name is the module id; its number is `order`), and copy the
  whole table into the quest README's course map with a status column (generated / not
  generated), so a later `/learn <topic>` can add the remaining rows.

## Example: `/learn tokio`

```
## Tokio — table of contents

From first runtime to production debugging, targeting tokio 1.47 (+ tokio-util 0.7, console-subscriber 0.4).

| # | module                  | you will be able to                                                     | lessons | needs     |
|---|-------------------------|-------------------------------------------------------------------------|---------|-----------|
| 1 | futures-and-tasks       | explain poll/Waker/Pin with a hand-written future, spawn vs join        | 4       | —         |
| 2 | runtimes-and-workers    | choose current_thread vs multi_thread and worker counts; prove blocking stalls | 4 | 1      |
| 3 | timers-and-cancellation | use sleep/timeout/interval/select! and cancel safely (drop semantics)   | 4       | 1         |
| 4 | channels-and-sync       | pick mpsc/oneshot/broadcast/watch and Mutex vs RwLock vs Semaphore      | 5       | 1         |
| 5 | async-io                | build a TCP echo server, framing with tokio-util codecs, backpressure   | 5       | 2, 4      |
| 6 | structured-concurrency  | JoinSet, graceful shutdown, CancellationToken                           | 3       | 3, 4      |
| 7 | observability           | find a blocked worker with tokio-console and tracing                    | 3       | 2         |

Suggested paths:
- **write services**: 1 → 2 → 3 → 5 → 6
- **debug slow async code**: 1 → 2 → 7

Which modules should I generate? (numbers, a path, or "all")
```

## Example: `/learn lance`

```
## Lance — table of contents

The Lance columnar format and dataset API, targeting lance 0.x (pin at generation time) and arrow 5x.

| # | module                 | you will be able to                                                         | lessons | needs                         |
|---|------------------------|-----------------------------------------------------------------------------|---------|-------------------------------|
| 1 | arrow-recordbatch      | build RecordBatch/Schema, slice zero-copy, stream with RecordBatchReader     | 4       | —                             |
| 2 | dataset-basics         | write/append/read a Lance dataset; versions and time travel                 | 4       | 1, NYC taxi (1.3 MB)          |
| 3 | scans-and-streams      | project/filter/limit with a scanner and prove only batches, not the table, are in memory | 4 | 2              |
| 4 | random-access          | take rows by index and compare cost vs Parquet with measured bytes          | 3       | 2                             |
| 5 | file-format-internals  | read pages/encodings of a Lance file and explain the layout                 | 4       | 2                             |
| 6 | vector-search          | build an IVF_PQ index and measure recall vs latency                         | 4       | 2, embeddings fixture (~50 MB)|

Suggested paths:
- **use Lance in an app**: 1 → 2 → 3 → 4
- **understand the format**: 1 → 2 → 5

Which modules should I generate? (numbers, a path, or "all")
```
