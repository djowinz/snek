mod game;
mod snake;
mod types;
mod ui;

use color_eyre::Result;

fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let dim_rec = terminal.size()?;
    let game_state = game::GameState::new(dim_rec.width - 2, dim_rec.height - 2);
    let result = game::run(terminal, game_state);
    ratatui::restore();
    result
}
