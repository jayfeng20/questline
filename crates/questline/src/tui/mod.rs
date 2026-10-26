//! The dashboard: terminal lifecycle, the event loop, and a worker thread that runs
//! evaluations so the UI stays responsive.

mod app;
mod theme;
mod ui;

pub use theme::Theme;

use std::io::Write;
use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;

use questline_core::EvalEvent;
use questline_core::Questline;
use questline_core::Snapshot;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event;
use ratatui::crossterm::event::Event;
use ratatui::crossterm::event::KeyEventKind;

use app::Action;
use app::App;
use app::Job;

enum Msg {
    Eval(EvalEvent),
    Done(Result<Snapshot, String>),
}

pub fn run(ql: Questline, theme: Theme) -> anyhow::Result<()> {
    let ql = Arc::new(ql);
    let snapshot = ql.sync()?;
    let (jobs_tx, jobs_rx) = mpsc::channel::<Job>();
    let (msg_tx, msg_rx) = mpsc::channel::<Msg>();
    let worker_ql = Arc::clone(&ql);
    std::thread::spawn(move || {
        for job in jobs_rx {
            let tx = msg_tx.clone();
            let result = worker_ql.evaluate(&job.targets, job.force, &mut |e| {
                tx.send(Msg::Eval(e)).ok();
            });
            if msg_tx
                .send(Msg::Done(result.map_err(|e| e.to_string())))
                .is_err()
            {
                break;
            }
        }
    });

    let mut app = App::new(ql, snapshot, jobs_tx, theme.palette());
    let mut terminal = ratatui::init();
    let result = event_loop(&mut terminal, &mut app, &msg_rx);
    ratatui::restore();
    result
}

fn event_loop(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    msgs: &mpsc::Receiver<Msg>,
) -> anyhow::Result<()> {
    loop {
        while let Ok(msg) = msgs.try_recv() {
            match msg {
                Msg::Eval(e) => app.on_eval(e),
                Msg::Done(result) => app.on_eval_done(result),
            }
        }
        terminal.draw(|frame| ui::draw(frame, app))?;
        // Short poll: live check output and flash expiry redraw without input.
        if !event::poll(Duration::from_millis(150))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        match app.on_key(key) {
            Action::None => {}
            Action::Quit => return Ok(()),
            Action::Suspend {
                dir,
                program,
                args,
                pause,
            } => {
                ratatui::restore();
                println!("$ cd {} && {program} {}", dir.display(), args.join(" "));
                let status = std::process::Command::new(&program)
                    .args(&args)
                    .current_dir(&dir)
                    .status();
                if let Err(e) = &status {
                    println!("could not run {program}: {e}");
                }
                if pause || status.is_err() {
                    print!("\n[press enter to return to questline]");
                    std::io::stdout().flush().ok();
                    let mut line = String::new();
                    std::io::stdin().read_line(&mut line).ok();
                }
                *terminal = ratatui::init();
                // A check run from the shell shows up right away.
                app.reload();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::crossterm::event::KeyCode;
    use ratatui::crossterm::event::KeyEvent;

    /// Renders the checked-in example quest through every view.
    #[test]
    fn every_view_renders_the_example_quest() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        let ql = Arc::new(Questline::open(&root).unwrap());
        let snapshot = ql.sync().unwrap();
        let (tx, _rx) = mpsc::channel();
        let mut app = App::new(ql, snapshot, tx, Theme::Default.palette());
        let mut terminal = Terminal::new(TestBackend::new(150, 30)).unwrap();
        let mut screen = |app: &mut App| {
            terminal.draw(|f| ui::draw(f, app)).unwrap();
            let buffer = terminal.backend().buffer();
            let text: Vec<String> = (0..buffer.area.height)
                .map(|y| {
                    (0..buffer.area.width)
                        .map(|x| buffer[(x, y)].symbol())
                        .collect::<String>()
                })
                .collect();
            if std::env::var_os("QUESTLINE_PRINT_SCREENS").is_some() {
                println!("{}\n", text.join("\n"));
            }
            text.join("\n")
        };
        let press = |app: &mut App, code| {
            app.on_key(KeyEvent::from(code));
        };

        app.command("example-arrow");
        let modules = screen(&mut app);
        assert!(modules.contains("Modules(example-arrow)"), "{modules}");
        assert!(modules.contains("arrays-and-recordbatch"));

        press(&mut app, KeyCode::Enter);
        let lessons = screen(&mut app);
        assert!(
            lessons.contains("Lessons(example-arrow/arrays-and-recordbatch)"),
            "{lessons}"
        );
        assert!(lessons.contains("typed_arrays_and_builders"));

        press(&mut app, KeyCode::Char('d'));
        let describe = screen(&mut app);
        assert!(describe.contains("Describe lesson"), "{describe}");

        press(&mut app, KeyCode::Esc);
        app.command("quests");
        let quests = screen(&mut app);
        assert!(quests.contains("Quests(all)"), "{quests}");

        press(&mut app, KeyCode::Char('?'));
        assert!(screen(&mut app).contains("drill down"));
    }
}
