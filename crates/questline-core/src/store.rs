//! `<root>/.questline/registry.parquet`: one row per module with its quest's metadata, its
//! lessons and its latest evaluation.
//!
//! The registry is a materialised view: manifests are the source of truth for metadata and
//! `.quest/progress.json` files for results, so a missing or outdated registry is rebuilt on
//! the next sync. What it adds is the cache that survives a deleted progress file, and one file
//! that tools can query (`duckdb -c "from '.questline/registry.parquet'"`).
//!
//! Writes replace the file atomically under an exclusive lock on `registry.lock`.

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use arrow_array::Array;
use arrow_array::ArrayRef;
use arrow_array::BooleanArray;
use arrow_array::ListArray;
use arrow_array::RecordBatch;
use arrow_array::StringArray;
use arrow_array::StructArray;
use arrow_array::TimestampMillisecondArray;
use arrow_array::UInt8Array;
use arrow_array::UInt32Array;
use arrow_array::cast::AsArray;
use arrow_array::types::TimestampMillisecondType;
use arrow_array::types::UInt8Type;
use arrow_buffer::OffsetBuffer;
use arrow_schema::DataType;
use arrow_schema::Field;
use arrow_schema::Fields;
use arrow_schema::Schema;
use arrow_schema::SchemaRef;
use arrow_schema::TimeUnit;
use parquet::arrow::ArrowWriter;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::file::metadata::KeyValue;
use parquet::file::properties::WriterProperties;
use questkit::ExerciseResult;
use questkit::ExerciseStatus;
use questkit::Outcome;
use questkit::Progress;

use crate::Error;
use crate::model::ModuleKey;
use crate::model::Quest;

pub const STATE_DIR: &str = ".questline";
const REGISTRY: &str = "registry.parquet";
const LOCK: &str = "registry.lock";
/// Bump on any schema change; an older file is then ignored and rewritten.
pub const SCHEMA_VERSION: &str = "1";
const VERSION_KEY: &str = "questline.registry.schema_version";

pub struct Store {
    dir: PathBuf,
}

/// Holds the registry lock until dropped.
pub struct Lock(#[allow(dead_code)] File);

impl Store {
    pub fn new(root: &Path) -> Self {
        Self {
            dir: root.join(STATE_DIR),
        }
    }

    pub fn path(&self) -> PathBuf {
        self.dir.join(REGISTRY)
    }

    pub fn lock(&self) -> Result<Lock, Error> {
        std::fs::create_dir_all(&self.dir)
            .map_err(|e| Error::io(format!("create {}", self.dir.display()), e))?;
        let path = self.dir.join(LOCK);
        let file =
            File::create(&path).map_err(|e| Error::io(format!("open {}", path.display()), e))?;
        file.lock()
            .map_err(|e| Error::io(format!("lock {}", path.display()), e))?;
        Ok(Lock(file))
    }

    /// The latest evaluation recorded per module; empty when there is no registry yet or it
    /// was written with another schema version.
    pub fn load_progress(&self) -> Result<HashMap<ModuleKey, Progress>, Error> {
        let path = self.path();
        let file = match File::open(&path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(HashMap::new()),
            Err(e) => return Err(Error::io(format!("open {}", path.display()), e)),
        };
        let builder = ParquetRecordBatchReaderBuilder::try_new(file).map_err(|e| self.error(e))?;
        let version = builder
            .metadata()
            .file_metadata()
            .key_value_metadata()
            .and_then(|kv| kv.iter().find(|k| k.key == VERSION_KEY))
            .and_then(|k| k.value.clone());
        if version.as_deref() != Some(SCHEMA_VERSION) {
            return Ok(HashMap::new());
        }
        let mut out = HashMap::new();
        for batch in builder.build().map_err(|e| self.error(e))? {
            let batch = batch.map_err(|e| self.error(e))?;
            read_progress(&batch, &mut out).ok_or_else(|| self.error("unexpected column types"))?;
        }
        Ok(out)
    }

    pub fn save(&self, quests: &[Quest], synced_at_ms: i64) -> Result<(), Error> {
        let batch = to_batch(quests, synced_at_ms).map_err(|e| self.error(e))?;
        let props = WriterProperties::builder()
            .set_key_value_metadata(Some(vec![KeyValue::new(
                VERSION_KEY.to_string(),
                SCHEMA_VERSION.to_string(),
            )]))
            .build();
        let path = self.path();
        let tmp = path.with_extension(format!("tmp{}", std::process::id()));
        let file =
            File::create(&tmp).map_err(|e| Error::io(format!("create {}", tmp.display()), e))?;
        let mut writer =
            ArrowWriter::try_new(file, batch.schema(), Some(props)).map_err(|e| self.error(e))?;
        writer.write(&batch).map_err(|e| self.error(e))?;
        writer.close().map_err(|e| self.error(e))?;
        std::fs::rename(&tmp, &path)
            .map_err(|e| Error::io(format!("replace {}", path.display()), e))
    }

    fn error(&self, e: impl std::fmt::Display) -> Error {
        Error::Registry(format!("{}: {e}", self.path().display()))
    }
}

fn utf8(name: &str, nullable: bool) -> Field {
    Field::new(name, DataType::Utf8, nullable)
}

fn timestamp(name: &str, nullable: bool) -> Field {
    Field::new(
        name,
        DataType::Timestamp(TimeUnit::Millisecond, Some("UTC".into())),
        nullable,
    )
}

fn list_of(name: &str, item: DataType) -> Field {
    Field::new(
        name,
        DataType::List(Arc::new(Field::new("item", item, false))),
        false,
    )
}

fn lesson_fields() -> Fields {
    Fields::from(vec![
        Field::new("number", DataType::UInt8, false),
        utf8("name", false),
        utf8("summary", false),
    ])
}

fn exercise_fields() -> Fields {
    Fields::from(vec![
        Field::new("lesson", DataType::UInt8, false),
        utf8("name", false),
        utf8("status", false),
        utf8("message", true),
    ])
}

pub fn schema() -> SchemaRef {
    let u32 = |name| Field::new(name, DataType::UInt32, false);
    Arc::new(Schema::new(vec![
        utf8("quest_id", false),
        utf8("quest_title", true),
        utf8("quest_summary", true),
        list_of("quest_tags", DataType::Utf8),
        Field::new("quest_example", DataType::Boolean, false),
        utf8("quest_path", false),
        utf8("module_id", false),
        utf8("module_title", true),
        utf8("module_summary", true),
        Field::new("module_order", DataType::UInt32, true),
        list_of("module_requires", DataType::Utf8),
        utf8("module_path", false),
        list_of("lessons", DataType::Struct(lesson_fields())),
        list_of("problems", DataType::Utf8),
        utf8("status", false),
        utf8("fingerprint", false),
        utf8("evaluated_fingerprint", true),
        utf8("outcome", true),
        utf8("message", true),
        timestamp("evaluated_at", true),
        u32("passed"),
        u32("failed"),
        u32("todo"),
        u32("total"),
        list_of("exercises", DataType::Struct(exercise_fields())),
        timestamp("synced_at", false),
    ]))
}

/// A list column from per-row item arrays' lengths and their concatenated values.
fn list(field: &Field, lengths: Vec<usize>, values: ArrayRef) -> ArrayRef {
    let DataType::List(item) = field.data_type() else {
        unreachable!("list_of fields only")
    };
    Arc::new(ListArray::new(
        item.clone(),
        OffsetBuffer::<i32>::from_lengths(lengths),
        values,
        None,
    ))
}

fn strings<'a>(values: impl IntoIterator<Item = Option<&'a str>>) -> ArrayRef {
    Arc::new(values.into_iter().collect::<StringArray>())
}

fn string_list(field: &Field, rows: &[Vec<&str>]) -> ArrayRef {
    let values = strings(rows.iter().flatten().map(|s| Some(*s)));
    list(field, rows.iter().map(Vec::len).collect(), values)
}

fn to_batch(quests: &[Quest], synced_at_ms: i64) -> Result<RecordBatch, arrow_schema::ArrowError> {
    let schema = schema();
    let f = |name: &str| schema.field_with_name(name).expect("schema field").clone();
    let modules: Vec<_> = quests
        .iter()
        .flat_map(|q| q.modules.iter().map(move |m| (q, m)))
        .collect();
    let progress: Vec<Option<&Progress>> =
        modules.iter().map(|(_, m)| m.progress.as_ref()).collect();
    let counts: Vec<_> = modules.iter().map(|(_, m)| m.counts()).collect();
    let lessons: Vec<&[questkit::Lesson]> = modules
        .iter()
        .map(|(_, m)| {
            m.manifest
                .as_ref()
                .map(|mm| mm.lessons.as_slice())
                .unwrap_or_default()
        })
        .collect();
    let exercises: Vec<&[ExerciseResult]> = progress
        .iter()
        .map(|p| p.map(|p| p.exercises.as_slice()).unwrap_or_default())
        .collect();

    let lesson_values: Vec<&questkit::Lesson> = lessons.iter().copied().flatten().collect();
    let lesson_structs = StructArray::try_new(
        lesson_fields(),
        vec![
            Arc::new(
                lesson_values
                    .iter()
                    .map(|l| l.number)
                    .collect::<UInt8Array>(),
            ),
            strings(lesson_values.iter().map(|l| Some(l.name.as_str()))),
            strings(lesson_values.iter().map(|l| Some(l.summary.as_str()))),
        ],
        None,
    )?;
    let exercise_values: Vec<&ExerciseResult> = exercises.iter().copied().flatten().collect();
    let exercise_structs = StructArray::try_new(
        exercise_fields(),
        vec![
            Arc::new(
                exercise_values
                    .iter()
                    .map(|e| e.lesson)
                    .collect::<UInt8Array>(),
            ),
            strings(exercise_values.iter().map(|e| Some(e.name.as_str()))),
            strings(
                exercise_values
                    .iter()
                    .map(|e| Some(exercise_status_str(e.status))),
            ),
            strings(exercise_values.iter().map(|e| e.message.as_deref())),
        ],
        None,
    )?;

    let path = |p: &Path| p.to_string_lossy().into_owned();
    let quest_paths: Vec<String> = modules.iter().map(|(q, _)| path(&q.path)).collect();
    let module_paths: Vec<String> = modules.iter().map(|(_, m)| path(&m.path)).collect();
    let empty = Vec::new();
    let tags: Vec<Vec<&str>> = modules
        .iter()
        .map(|(q, _)| {
            q.manifest
                .as_ref()
                .map_or(&empty, |m| &m.tags)
                .iter()
                .map(String::as_str)
                .collect()
        })
        .collect();
    let requires: Vec<Vec<&str>> = modules
        .iter()
        .map(|(_, m)| {
            m.manifest
                .as_ref()
                .map_or(&empty, |mm| &mm.requires)
                .iter()
                .map(String::as_str)
                .collect()
        })
        .collect();
    let problems: Vec<Vec<&str>> = modules
        .iter()
        .map(|(q, m)| {
            q.problems
                .iter()
                .chain(&m.problems)
                .map(String::as_str)
                .collect()
        })
        .collect();
    let u32s = |get: fn(&questkit::Counts) -> u32| {
        Arc::new(counts.iter().map(get).collect::<UInt32Array>()) as ArrayRef
    };

    let columns: Vec<ArrayRef> = vec![
        strings(modules.iter().map(|(q, _)| Some(q.id.as_str()))),
        strings(
            modules
                .iter()
                .map(|(q, _)| q.manifest.as_ref().map(|m| m.title.as_str())),
        ),
        strings(
            modules
                .iter()
                .map(|(q, _)| q.manifest.as_ref().map(|m| m.summary.as_str())),
        ),
        string_list(&f("quest_tags"), &tags),
        Arc::new(
            modules
                .iter()
                .map(|(q, _)| Some(q.manifest.as_ref().is_some_and(|m| m.example)))
                .collect::<BooleanArray>(),
        ),
        strings(quest_paths.iter().map(|p| Some(p.as_str()))),
        strings(modules.iter().map(|(_, m)| Some(m.key.module.as_str()))),
        strings(
            modules
                .iter()
                .map(|(_, m)| m.manifest.as_ref().map(|mm| mm.title.as_str())),
        ),
        strings(
            modules
                .iter()
                .map(|(_, m)| m.manifest.as_ref().map(|mm| mm.summary.as_str())),
        ),
        Arc::new(
            modules
                .iter()
                .map(|(_, m)| m.manifest.as_ref().map(|mm| mm.order))
                .collect::<UInt32Array>(),
        ),
        string_list(&f("module_requires"), &requires),
        strings(module_paths.iter().map(|p| Some(p.as_str()))),
        list(
            &f("lessons"),
            lessons.iter().map(|l| l.len()).collect(),
            Arc::new(lesson_structs),
        ),
        string_list(&f("problems"), &problems),
        strings(modules.iter().map(|(_, m)| Some(status_str(m.status())))),
        strings(modules.iter().map(|(_, m)| Some(m.fingerprint.as_str()))),
        strings(progress.iter().map(|p| p.map(|p| p.fingerprint.as_str()))),
        strings(progress.iter().map(|p| p.map(|p| outcome_str(p.outcome)))),
        strings(
            progress
                .iter()
                .map(|p| p.and_then(|p| p.message.as_deref())),
        ),
        Arc::new(
            progress
                .iter()
                .map(|p| p.map(|p| p.evaluated_at_ms))
                .collect::<TimestampMillisecondArray>()
                .with_timezone("UTC"),
        ),
        u32s(|c| c.passed),
        u32s(|c| c.failed),
        u32s(|c| c.todo),
        u32s(questkit::Counts::total),
        list(
            &f("exercises"),
            exercises.iter().map(|e| e.len()).collect(),
            Arc::new(exercise_structs),
        ),
        Arc::new(
            TimestampMillisecondArray::from(vec![synced_at_ms; modules.len()]).with_timezone("UTC"),
        ),
    ];
    RecordBatch::try_new(schema, columns)
}

fn read_progress(batch: &RecordBatch, out: &mut HashMap<ModuleKey, Progress>) -> Option<()> {
    let col = |name| batch.column_by_name(name);
    let quest = col("quest_id")?.as_string_opt::<i32>()?;
    let module = col("module_id")?.as_string_opt::<i32>()?;
    let fingerprint = col("evaluated_fingerprint")?.as_string_opt::<i32>()?;
    let outcome = col("outcome")?.as_string_opt::<i32>()?;
    let message = col("message")?.as_string_opt::<i32>()?;
    let evaluated_at = col("evaluated_at")?.as_primitive_opt::<TimestampMillisecondType>()?;
    let exercises = col("exercises")?.as_list_opt::<i32>()?;
    for row in 0..batch.num_rows() {
        if fingerprint.is_null(row) {
            continue;
        }
        let items = exercises.value(row);
        let items = items.as_struct_opt()?;
        let lesson = items.column(0).as_primitive_opt::<UInt8Type>()?;
        let name = items.column(1).as_string_opt::<i32>()?;
        let status = items.column(2).as_string_opt::<i32>()?;
        let msg = items.column(3).as_string_opt::<i32>()?;
        let results = (0..items.len())
            .map(|i| {
                Some(ExerciseResult {
                    lesson: lesson.value(i),
                    name: name.value(i).to_string(),
                    status: parse_exercise_status(status.value(i))?,
                    message: msg.is_valid(i).then(|| msg.value(i).to_string()),
                })
            })
            .collect::<Option<Vec<_>>>()?;
        let mut progress = Progress::new(
            fingerprint.value(row).to_string(),
            parse_outcome(outcome.value(row))?,
            results,
        );
        progress.evaluated_at_ms = evaluated_at.value(row);
        progress.message = message
            .is_valid(row)
            .then(|| message.value(row).to_string());
        out.insert(
            ModuleKey::new(quest.value(row), module.value(row)),
            progress,
        );
    }
    Some(())
}

fn status_str(status: crate::Status) -> &'static str {
    use crate::Status::*;
    match status {
        Invalid => "invalid",
        Pending => "pending",
        Stale => "stale",
        BuildFailed => "build_failed",
        NotStarted => "not_started",
        InProgress => "in_progress",
        Completed => "completed",
    }
}

fn outcome_str(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Tested => "tested",
        Outcome::BuildFailed => "build_failed",
    }
}

fn parse_outcome(s: &str) -> Option<Outcome> {
    [Outcome::Tested, Outcome::BuildFailed]
        .into_iter()
        .find(|o| outcome_str(*o) == s)
}

fn exercise_status_str(status: ExerciseStatus) -> &'static str {
    match status {
        ExerciseStatus::Passed => "passed",
        ExerciseStatus::Failed => "failed",
        ExerciseStatus::Todo => "todo",
        ExerciseStatus::Ignored => "ignored",
    }
}

fn parse_exercise_status(s: &str) -> Option<ExerciseStatus> {
    use ExerciseStatus::*;
    [Passed, Failed, Todo, Ignored]
        .into_iter()
        .find(|st| exercise_status_str(*st) == s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover;

    #[test]
    fn save_then_load_round_trips_progress_and_keeps_metadata() {
        let root = tempfile::tempdir().unwrap();
        let module_dir = discover::tests::write_quest(root.path(), "arrow", "basics", 2);
        discover::tests::write_quest(root.path(), "arrow", "unchecked", 1);
        let mut progress = Progress::new(
            "f00d".into(),
            Outcome::Tested,
            vec![
                ExerciseResult {
                    lesson: 1,
                    name: "e01::a".into(),
                    status: ExerciseStatus::Passed,
                    message: None,
                },
                ExerciseResult {
                    lesson: 2,
                    name: "e02::b".into(),
                    status: ExerciseStatus::Failed,
                    message: Some("left: 1".into()),
                },
            ],
        );
        progress.message = Some("note".into());
        progress.save(&module_dir).unwrap();

        let quests = discover::scan(root.path()).unwrap();
        let store = Store::new(root.path());
        let _lock = store.lock().unwrap();
        store.save(&quests, 1_700_000_000_000).unwrap();
        let loaded = store.load_progress().unwrap();
        assert_eq!(loaded.len(), 1, "only evaluated modules carry progress");
        assert_eq!(loaded[&ModuleKey::new("arrow", "basics")], progress);

        let file = File::open(store.path()).unwrap();
        let batch = ParquetRecordBatchReaderBuilder::try_new(file)
            .unwrap()
            .build()
            .unwrap()
            .next()
            .unwrap()
            .unwrap();
        assert_eq!(batch.num_rows(), 2);
        assert_eq!(batch.schema(), schema());
        let lessons = batch.column_by_name("lessons").unwrap().as_list::<i32>();
        assert_eq!(lessons.value_length(0), 2);
        let status = batch.column_by_name("status").unwrap().as_string::<i32>();
        assert_eq!(
            status.value(0),
            "stale",
            "the progress was written for other inputs"
        );
        assert_eq!(status.value(1), "pending");
    }

    #[test]
    fn a_registry_from_another_schema_version_is_ignored() {
        let root = tempfile::tempdir().unwrap();
        let store = Store::new(root.path());
        std::fs::create_dir_all(root.path().join(STATE_DIR)).unwrap();
        let batch = RecordBatch::try_new(
            Arc::new(Schema::new(vec![utf8("quest_id", false)])),
            vec![strings([Some("old")])],
        )
        .unwrap();
        let mut writer =
            ArrowWriter::try_new(File::create(store.path()).unwrap(), batch.schema(), None)
                .unwrap();
        writer.write(&batch).unwrap();
        writer.close().unwrap();
        assert!(store.load_progress().unwrap().is_empty());
    }
}
