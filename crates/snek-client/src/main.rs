mod app;
mod game;
mod network;
mod screens;
mod snake;
mod ui;

use color_eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;

    // Accept optional relay URL: cargo run -p snek-client -- ws://host:port
    let relay_url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "ws://127.0.0.1:9090".to_string());

    let terminal = ratatui::init();
    let result = app::run(terminal, &relay_url);
    ratatui::restore();
    result
}
