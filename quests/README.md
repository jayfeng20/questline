# quests/

Every directory here with a `quest.toml` is a registered quest: questline lists it on its next
sync, with nothing else to update. A quest is a Cargo workspace whose member crates under
`modules/` are its sub-courses; [`docs/contract.md`](../docs/contract.md) is the full contract,
and `questline validate` checks a quest against it.

Write quests with the `/learn` skill ([`skills/learn`](../skills/learn)), which places each topic
into the quest on the same subject or starts a new one. Everything in this directory except
`example-arrow/` is gitignored, so your own quests and progress stay local.
