//! A runnable course. Each module under [`lessons`] is one lesson; each module under
//! [`exercises`] is its homework, checked by `tests/exercises/`.
//!
//! How to read a lesson file:
//!   * the module doc comment is the lecture: goal, mental model, reading list;
//!   * `// STEP n:` comments walk through the code, `📖` points at the docs/source to read;
//!   * `check(..)` lines are claims verified while the lesson runs (`✅`);
//!   * `try_it(..)` lines are experiments: change the code, re-run, compare.

pub mod exercises;
pub mod lessons;
pub mod util;
