# Miyu for Mac

A terminal AI assistant for macOS. Miyu lives in your shell, intercepts unknown commands, streams LLM responses in real time, and maintains conversation context across your terminal session.

## Features

- **Shell Integration** — type anything that isn't a command, Miyu responds
- **Session Context** — same terminal window = same conversation, stored as readable JSON
- **Streaming Output** — braille spinner, colored reasoning/content separation
- **15 Built-in Tools** — file I/O, web search, weather, calculator, Homebrew, and more
- **TUI Config** (`miyu config`) — manage LLM providers and models interactively
- **OpenAI-Compatible** — works with any provider that speaks the chat completions API
- **Single Binary** — ~5 MB, no runtime dependencies

## Quick Start

### Install via Homebrew

```bash
brew install orangerin/apps/miyu4mac
```

### Or build from source

```bash
git clone https://github.com/Shiorangerin/Miyu-For-Mac.git
cd Miyu-For-Mac
cargo build --release
./target/release/miyu4mac --version
```

### Enable shell integration

```bash
miyu fish-init    # fish shell (recommended)
miyu bash-init    # bash
miyu zsh-init     # zsh
```

After installation, restart your shell. Now typing anything that isn't a valid command automatically goes to Miyu:

```fish
$ what's the weather in Tokyo
Tokyo: ☀️ +28°C

$ and what about tomorrow?
Tokyo tomorrow: ⛅ +25°C  ← remembers the city from context!
```

## Usage

```bash
miyu                  # Interactive REPL
miyu "Hello"          # Single message
miyu config           # TUI configuration editor
miyu paths            # Show config file locations
miyu session          # Show current session info
```

### REPL Commands

| Command | Description |
|---|---|
| `/help` | Show help |
| `/clear` | Clear conversation history |
| `/mode normal\|chat\|plan` | Switch mode |
| `/exit` | Quit |

**Modes:**
- **normal** — full tool access (default)
- **chat** — pure conversation, no tools
- **plan** — tools shown but not executed

## Configuration

Run `miyu config` for an interactive TUI, or edit `~/.miyu_macos.json` directly.

### Environment Variables

| Variable | Default | Description |
|---|---|---|
| `MIYU_WORKDIR` | `$PWD` | Working directory for file operations |
| `MIYU_PERSONA` | `~/.miyu/persona.md` | Custom system prompt (Markdown) |

### Adding LLM Providers

Use `miyu config` → "Manage Providers" to add any OpenAI-compatible endpoint:

- opencode Zen (default, no key)
- OpenAI, Anthropic, DeepSeek, Gemini
- Ollama (`http://localhost:11434/v1`)
- Any custom endpoint

### Custom Persona

Create `~/.miyu/persona.md` with your desired system prompt. Example:

```markdown
You are a helpful macOS terminal assistant. 
Keep responses concise and actionable.
```

## Tools

| Tool | Description |
|---|---|
| `run_command` | Execute shell commands (sandboxed) |
| `read_file` | Read files with line numbers |
| `write_file` | Write files (sandbox-restricted) |
| `edit_file` | Find-and-replace in files |
| `web_search` | DuckDuckGo search |
| `web_fetch` | Fetch web page content |
| `list_directory` | List files in a directory |
| `search_files` | Find files by name pattern |
| `search_content` | Regex grep in files |
| `system_info` | macOS system diagnostics |
| `weather` | Weather via wttr.in |
| `calculator` | Safe math expression eval |
| `brew_info` | Homebrew package info |
| `todowrite` | Structured task tracking |
| `ask_question` | User clarification |

## Session Storage

Conversations are stored per terminal session at `~/.miyu/sessions/<tty>.json`:

```json
{
  "id": "ttys001",
  "messages": [
    {"role": "system", "content": "..."},
    {"role": "user", "content": "what is rust"},
    {"role": "assistant", "content": "Rust is a systems programming language..."}
  ],
  "persona_name": "Miyu"
}
```

This format is human-readable and can be inspected by other tools.

## Project Structure

```
src/
├── main.rs         Entry point, CLI, shell intercept mode
├── config.rs       Environment-based configuration
├── config_tui.rs   Interactive TUI config editor (730 lines)
├── session.rs      Per-TTY conversation storage
├── persona.rs      System prompt management
├── i18n.rs         Bilingual support (zh/en)
├── llm.rs          OpenAI-compatible streaming client
├── render.rs       Terminal output + braille spinner
├── repl.rs         Interactive REPL loop
└── tools.rs        15 built-in tools (901 lines)
```

## Requirements

- macOS (Apple Silicon or Intel)
- Rust 1.70+ (build from source only)
- No other dependencies — single static binary

## Acknowledgments

Inspired by [SHORiN-KiWATA/Miyu](https://github.com/SHORiN-KiWATA/Miyu), the original terminal AI assistant for Linux.

## License

MIT
