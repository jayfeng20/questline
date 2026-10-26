# questify — an agent skill that writes questline quests

`/questify <topic>` writes a **module**, a runnable, test-checked Rust course, into the quest on the
same subject under `quests/` (or starts a new quest), so questline registers it and tracks its
progress. A module has lessons you run (`cargo run -- 1`), exercises you complete
(`src/exercises/`), tests that check you and record progress (`cargo run -- check 1`), reference
solutions, real fixtures in `data/`, and references for when you are stuck. Broad topics first
get a table of contents to pick from.

Placement: by default (**autosort**, `--auto`) a topic joins the quest that already covers its
subject and shares that workspace's dependency pins: `/questify arrow-parquet` adds a module to the
`arrow-rs` quest; with no matching quest it creates one. **Manualsort** (`/questify --manual
<topic>`) asks before creating anything. See [`references/placement.md`](references/placement.md).

The format follows the Agent Skills standard (`SKILL.md` + YAML frontmatter), so the same folder
works in Claude Code, Codex and other agents.

## Install

From a questline checkout (the skill finds `quests/` relative to itself; set `QUESTLINE_ROOT` to
use another checkout):

```bash
# Claude Code (user-level; invoke with /questify <topic>)
ln -s "$PWD/skills/questify" ~/.claude/skills/questify
# Codex CLI (invoke with $questify <topic>, or just ask it to teach you a topic)
mkdir -p ~/.codex/skills && ln -s "$PWD/skills/questify" ~/.codex/skills/questify
# Other agents that read ~/.agents/skills
mkdir -p ~/.agents/skills && ln -s "$PWD/skills/questify" ~/.agents/skills/questify
```

## Layout

| path                          | purpose                                                          |
|-------------------------------|------------------------------------------------------------------|
| `SKILL.md`                    | the workflow the agent follows                                    |
| `references/course-spec.md`   | the exact shape of a module, a lesson, an exercise, a test        |
| `references/placement.md`     | which quest a topic goes into: autosort and manualsort            |
| `references/toc-format.md`    | how to scope a broad topic into a course map and ask which modules to write |
| `references/examples.md`      | lesson archetypes → which verified example to imitate             |
| `examples/`                   | a verified example quest (not registered): `modules/std-and-tokio` (Vec growth, tokio workers, Arrow streaming, HashMap and Futures from scratch), `modules/lance-dataset` (Lance Dataset, fragments, Scanner, take, versions with IO evidence) |
| `templates/quest/`            | a quest workspace skeleton                                        |
| `templates/module/`           | a module skeleton (compiles and passes its contract as-is)        |
| `scripts/questline-root.sh`   | print the questline root                                          |
| `scripts/quests.sh`           | list every quest's manifest, pins and modules, for placement      |
| `scripts/scaffold.sh`         | create a quest or a module from the templates                     |

The contract every module satisfies is [`docs/contract.md`](../../docs/contract.md).
