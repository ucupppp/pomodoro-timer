use color_eyre::{eyre::Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout}, style::{Color, Style, Stylize}, text::{Line, Span}, widgets::{Block, BorderType, Borders, Gauge, Paragraph}, DefaultTerminal, Frame
};
use std::{env, process::abort, time::{Duration, Instant}};
use rodio::{Decoder, OutputStreamBuilder, Sink, self, source::Source, source::SineWave, OutputStream};
use std::{fs::File, io::BufReader};


struct AppState {
    start_time: Instant,
    duration: Duration,
    paused: bool,
    alarm_played: bool,
    audio_stream: Option<OutputStream>,
    sink: Option<rodio::Sink>,
    paused_at: Option<Instant>,  // new field
}

fn main() -> Result<()> {
    let args: Vec<String> =  env::args().collect();
    if args.len() < 2 {
        println!("Provide timer duration in second!");
        abort() 
    }

    let dur: u64 = args[1].parse().unwrap();

    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = run(terminal, dur);
    ratatui::restore();
    result
}

fn run(mut terminal: DefaultTerminal, mut dur :u64) -> Result<()> {
    if dur == 0 {
        dur = 10
    }

    let mut app = AppState {
        start_time: Instant::now(),
        duration: Duration::from_secs(dur),
        paused: false,
        alarm_played: false,
        audio_stream: None,
        sink:None,
        paused_at:None,
    };

    loop {
        terminal.draw(|frame| draw(frame, &app))?;

        // Hitung elapsed & cek alarm
        let elapsed = if app.paused {
            app.paused_at.unwrap().duration_since(app.start_time)
        } else {
            Instant::now().duration_since(app.start_time)
        };

        let total = app.duration;

        if !app.paused && elapsed >= total && !app.alarm_played {
            app.play_beep(); // gunakan versi yang simpan stream
            app.alarm_played = true;
        }

        std::thread::sleep(Duration::from_millis(100));

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => break Ok(()),
                        KeyCode::Char('r') => {
                            app.start_time = Instant::now();
                            app.paused = false;
                            app.alarm_played = false;
                            app.sink = None;
                            app.audio_stream = None;
                        }
                        KeyCode::Char(' ') => {
                            if app.paused {
                                if let Some(paused_instant) = app.paused_at {
                                    let paused_duration = Instant::now().duration_since(paused_instant);
                                    app.start_time += paused_duration;
                                }
                                app.paused = false;
                                app.paused_at = None;
                            } else {
                                app.paused_at = Some(Instant::now());
                                app.paused = true;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

fn draw(frame: &mut Frame, app: &AppState) {
    // Hitung waktu sisa
    let elapsed = if app.paused {
        app.paused_at.unwrap().duration_since(app.start_time)
    } else {
        Instant::now().duration_since(app.start_time)
    };
    let total = app.duration;
    let progress = if elapsed >= total {
        1.0
    } else {
        elapsed.as_secs_f64() / total.as_secs_f64()
    };

    // Hitung sisa menit:detik
    let remaining = if elapsed >= total {
        Duration::ZERO
    } else {
        total - elapsed
    };
    let mins = remaining.as_secs() / 60;
    let secs = remaining.as_secs() % 60;

    let time_display = format!("{:02}:{:02}", mins, secs);

    let paragraph = Paragraph::new(time_display.clone());

    let gauge = Gauge::default()
        .block(
            Block::default()
            .title("Progress")
            .borders(Borders::ALL)
            .title_bottom(
                Line::from(vec![
                    Span::raw("Quit "),
                    Span::styled("<Q>", Style::default().fg(Color::Red)),
                ])
            )
            .title_bottom(
                Line::from(vec![
                    Span::raw("Pause "),
                    Span::styled("<Space>", Style::default().fg(Color::Red)),
                ])
            )
            .title_bottom(
                Line::from(vec![
                    Span::raw("Reset "),
                    Span::styled("<R>", Style::default().fg(Color::Red)),
                ])
            )
        )
        .gauge_style(Style::default().fg(Color::Green))
        .ratio(progress);

    let outer_block = Block::default()
        .border_type(BorderType::Rounded)
        .title(Line::from(" Pomodoro Timer ").centered())
        .borders(Borders::ALL);

    let area = frame.size();
    frame.render_widget(outer_block.clone(), area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([Constraint::Length(3), Constraint::Length(3)].as_ref())
        .split(area);

    frame.render_widget(paragraph.block(Block::default().title("Time")), chunks[0]);
    frame.render_widget(gauge, chunks[1]);
}



fn play_alarm() -> anyhow::Result<()> {
    // Buka stream audio ke perangkat default
    let handle = OutputStreamBuilder::open_default_stream().expect("blablabla");
    let sink = Sink::connect_new(&handle.mixer());

    // Baca file audio
    let file = File::open("alarm.wav")?;
    let source = Decoder::new(BufReader::new(file))?;

    sink.append(source);
    sink.detach(); // biar main thread gak nunggu

    Ok(())
}

fn play_beep() -> anyhow::Result<()> {
    let handle = OutputStreamBuilder::open_default_stream().expect("blablabla");
    let sink = Sink::connect_new(&handle.mixer());

    let src = SineWave::new(540.0).take_duration(Duration::from_millis(500));
    sink.append(src);
    sink.detach();

    Ok(())
}

impl AppState {
    fn play_beep(&mut self) {
        if self.sink.is_some() {
            return;
        }

        if let Ok(mut handle) = OutputStreamBuilder::open_default_stream() {
            let sink = Sink::connect_new(&handle.mixer());
            let src = SineWave::new(440.0)
                .take_duration(Duration::from_millis(500));
            sink.append(src);
            handle.log_on_drop(false);
            self.audio_stream = Some(handle);
            self.sink = Some(sink);
        }
    }
}

