# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run Commands

- **Build:** `cargo build`
- **Run:** `cargo run`
- **Check (fast compile check):** `cargo check`
- **Lint:** `cargo clippy`
- **Format:** `cargo fmt`
- **Format check:** `cargo fmt -- --check`

No tests exist yet. The project has no workspace — it's a single binary crate.

## Architecture

This is a terminal Snake game ("snek") built as a single-file Rust binary (`src/main.rs`) using Ratatui for TUI rendering and crossterm for input handling.

**Core types:**
- `GameState` — holds all game state: snake body (`VecDeque<SpacePoint>`), food position, direction, score, speed, and alive/running/winner flags. Contains `update()` for tick logic and `generate_food()` for random placement.
- `SpacePoint` — simple `{x, y}` coordinate used for snake segments and food.
- `KeyDirection` — enum for movement direction (Up/Down/Left/Right).
- `PauseMenu` — a Ratatui widget struct (defined but not yet wired into rendering).

**Rendering:** `GameState` implements `Widget`, rendering the snake, food, border, score, and instructions directly to the Ratatui buffer using cell-level access.

**Game loop:** `run()` uses `crossterm::event::poll` with the game speed as timeout for non-blocking input, then updates state and redraws each tick. The snake wraps around screen edges. Speed increases as the snake eats food.

## Dependencies

- `ratatui` + `crossterm` — TUI framework and terminal backend
- `color-eyre` — error handling (`Result` type used throughout)
- `rand` — food placement RNG
- `derive_setters` — derive macro for builder-style setters (used on `PauseMenu`)

## Notes

- Rust 2024 edition
- The game board dimensions are derived from terminal size minus border (width-2, height-2)
