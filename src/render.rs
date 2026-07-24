use crate::llm::StreamChunk;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const BRAILLE: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub struct StreamRenderer {
    wrote_content: bool,
    spinner: Arc<Mutex<SpinnerState>>,
    show_reasoning: bool,
}

struct SpinnerState {
    active: bool,
    text: String,
}

impl StreamRenderer {
    pub fn new() -> Self {
        Self {
            wrote_content: false,
            spinner: Arc::new(Mutex::new(SpinnerState {
                active: false,
                text: String::new(),
            })),
            show_reasoning: true,
        }
    }

    pub fn new_with_reasoning(show: bool) -> Self {
        Self {
            wrote_content: false,
            spinner: Arc::new(Mutex::new(SpinnerState {
                active: false,
                text: String::new(),
            })),
            show_reasoning: show,
        }
    }

    pub fn write_chunk(&mut self, chunk: &StreamChunk) -> io::Result<()> {
        self.hide_spinner();
        let mut stdout = io::stdout();
        match chunk {
            StreamChunk::Content(text) => {
                write!(stdout, "{text}")?;
                self.wrote_content = true;
            }
            StreamChunk::Reasoning(text) => {
                if self.show_reasoning {
                    write!(stdout, "\x1b[2m\x1b[32m{text}\x1b[0m")?;
                    self.wrote_content = true;
                }
            }
        }
        stdout.flush()?;
        Ok(())
    }

    pub fn show_spinner(&self, text: &str) {
        let mut state = self.spinner.lock().unwrap();
        if state.active {
            return;
        }
        state.active = true;
        state.text = text.to_string();
        drop(state);

        let spinner = self.spinner.clone();
        std::thread::spawn(move || {
            let mut i = 0usize;
            while spinner.lock().unwrap().active {
                let t = spinner.lock().unwrap().text.clone();
                let mut stderr = io::stderr();
                let _ = write!(
                    stderr,
                    "\r\x1b[2m\x1b[36m{} {}\x1b[0m  ",
                    BRAILLE[i % BRAILLE.len()],
                    t
                );
                let _ = stderr.flush();
                i += 1;
                std::thread::sleep(Duration::from_millis(100));
            }
            let mut stderr = io::stderr();
            let _ = write!(stderr, "\r\x1b[K");
            let _ = stderr.flush();
        });
    }

    pub fn hide_spinner(&self) {
        self.spinner.lock().unwrap().active = false;
        std::thread::sleep(Duration::from_millis(150));
    }

    pub fn write_tool_call(&mut self, name: &str) -> io::Result<()> {
        self.hide_spinner();
        let mut stdout = io::stdout();
        writeln!(stdout, "\n\x1b[2m  ↳ {name}\x1b[0m")?;
        stdout.flush()?;
        self.wrote_content = true;
        Ok(())
    }

    pub fn write_tool_result(&mut self, ok: bool, brief: &str) -> io::Result<()> {
        let mut stdout = io::stdout();
        let status = if ok {
            "\x1b[32mok\x1b[0m"
        } else {
            "\x1b[31merr\x1b[0m"
        };
        writeln!(stdout, "\x1b[2m    [{status}] {brief}\x1b[0m")?;
        stdout.flush()?;
        Ok(())
    }

    pub fn finish(&mut self) -> io::Result<()> {
        self.hide_spinner();
        if self.wrote_content {
            writeln!(io::stdout())?;
            io::stdout().flush()?;
            self.wrote_content = false;
        }
        Ok(())
    }

    pub fn print_token_usage(&self, turn: u64, session: u64) -> io::Result<()> {
        self.hide_spinner();
        let mut stdout = io::stdout();
        writeln!(
            stdout,
            "\x1b[2m[Tokens: {turn} this turn / {session} session]\x1b[0m"
        )?;
        stdout.flush()?;
        Ok(())
    }
}
