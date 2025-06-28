// examples/terminal-ui/src/main.rs
use kryon_runtime::{KryonApp, RatatuiRenderer};
use ratatui::{
    backend::CrosstermBackend,
};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    io::{self, Stdout},
    time::{Duration, Instant},
};
use anyhow::Result;

type Terminal = ratatui::Terminal<CrosstermBackend<Stdout>>;

fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Create app
    let renderer = RatatuiRenderer::initialize(terminal.backend_mut().clone())?;
    let mut app = KryonApp::new("test-files/sample.krb", renderer)?;
    
    // Run app
    let result = run_app(&mut terminal, &mut app);
    
    // Cleanup terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    
    result
}

fn run_app(terminal: &mut Terminal, app: &mut KryonApp<RatatuiRenderer<CrosstermBackend<Stdout>>>) -> Result<()> {
    let mut last_frame_time = Instant::now();
    
    loop {
        // Handle input
        if event::poll(Duration::from_millis(16))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Esc => break,
                    _ => {
                        // Convert crossterm events to Kryon events
                        // TODO: Implement proper event conversion
                    }
                }
            }
        }
        
        // Update app
        let now = Instant::now();
        let delta_time = now.duration_since(last_frame_time);
        last_frame_time = now;
        
        app.update(delta_time)?;
        app.render()?;
        
        std::thread::sleep(Duration::from_millis(16)); // ~60 FPS
    }
    
    Ok(())
}