use crossterm::{
    cursor::{self, MoveTo},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    style::{
        Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
    },
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Stdout, Write};
use std::path::Path;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const INDICATOR_COUNT: usize = 6;
const START_STATE: [NodeColor; INDICATOR_COUNT] = [NodeColor::Off; INDICATOR_COUNT];
const TARGET_STATE: [NodeColor; INDICATOR_COUNT] = [
    NodeColor::White,
    NodeColor::Purple,
    NodeColor::Green,
    NodeColor::White,
    NodeColor::Purple,
    NodeColor::Green,
];

const SPLASH_LOGO: &str = r#"
..=%@@@@@@@@@@*-..
                                          .+%@@@@@@@@@@@--@@@@@#-.
                                      .-#@@@@@@@@@@@@@@@--@@@@@@@@.
                                    :%@@@@@@@@@@@@@@@@@@@@@@@@@@@@.
                                  -%@@@@@@@@@@@@@@@@@@@@@@@@@@@#-.-%=.
                                :%@@@@@@@@@@@@@@@@@@@@@@@@%=:.. :%@@@%.
                              .=@@@@@@@@@@@@@%+.           =++%@@@@@@@@-
                             .+@@@@@@@@@@@@+                -@@@@@@@@@@@=
                             =@@@@@@@@@@@#.                  .=@@@@@@@@@@-
                            :@@@@@@@@@@@=                      .@@@@@@@@@@:
                           .#@@@@@@@@@@#        HACK THE WORLD  -@@@@@@@@@*.
                           :%@@@@@@@@@@                          *@@@@@@@@@.
                           -@@@@@@@@@@#                          .@@@@@@@@@:
                           -@@@@@@@@@@#                          .@@@@@@@@@:
                           :%@@@@@@@@@@                          +@@@@@@@@@.
                        .  .*@@@@@@@@@@+                        :%@@@@@@@@#.
                        .=  .@@@@@@@@@@@-                       #@@@@@@@@@=
                         #:  :@@@@@@@@@@@=                    :@@@@@@@@@@+.
                         +@.  -@@@@@@@@@@@%-                .+@@@@@@@@@@=
                         :%@-  .#@@@@@@@@@@@@*            =%@@@@@@@@@@@-
                          =@@*.  -%@@@@@@@@@@@@@@%*==+#@@@@@@@@@@@@@@+.
                           =@@@:   :@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@+.
                           .+@@@%=   .=#@@@@@@@@@@@@@@@@@@@@@@@@*:
                             :@@@@@#.    .=@@@@@@@@@@@@@@@@@#:.   .:**=..
                              .*@@@@@@*-.    ..:-=====-:..     .+%@@@@@@@+
                                .#@@@@@@@@#-:.           ..:=%@@@@@@%#*@@@=
                                  .=@@@@@@@@@@@@@%%%%%%@@@@@@@@@@@-.   *@@=
                                     .+%@@@@@@@@@@@@@@@@@@@@@@%+.  .-+@@@=.
                                        ..:+%@@@@@@@@@@@@#=:..     .:--..
"#;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum NodeColor {
    Off,
    Green,
    Blue,
    Red,
    Purple,
    White,
}

impl NodeColor {
    fn next(self) -> Self {
        match self {
            Self::Off => Self::Green,
            Self::Green => Self::Blue,
            Self::Blue => Self::Red,
            Self::Red => Self::Purple,
            Self::Purple => Self::White,
            Self::White => Self::Off,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Off => "OFF",
            Self::Green => "GREEN",
            Self::Blue => "BLUE",
            Self::Red => "RED",
            Self::Purple => "PURPLE",
            Self::White => "WHITE",
        }
    }

    fn term_color(self) -> Color {
        match self {
            Self::Off => Color::DarkGrey,
            Self::Green => Color::Green,
            Self::Blue => Color::Blue,
            Self::Red => Color::Red,
            Self::Purple => Color::Magenta,
            Self::White => Color::White,
        }
    }
}

#[derive(Clone, Copy)]
enum AppPhase {
    Puzzle,
    Email,
    Submitted,
}

#[derive(Clone, Copy)]
enum PuzzleFocus {
    Indicator(usize),
    Action(usize),
}

#[derive(Clone, Copy)]
enum EmailFocus {
    Input,
    Buttons,
}

struct PuzzleState {
    initial: [NodeColor; INDICATOR_COUNT],
    current: [NodeColor; INDICATOR_COUNT],
    optimal_moves: usize,
    moves_taken: usize,
    focus: PuzzleFocus,
    show_rules: bool,
    status: String,
}

struct EmailState {
    email: String,
    focus: EmailFocus,
    selected_button: usize,
    status: String,
}

struct App {
    phase: AppPhase,
    puzzle: PuzzleState,
    email: EmailState,
    submitted_email: Option<String>,
    debug: bool,
    should_quit: bool,
}

struct TerminalSession;

impl TerminalSession {
    fn enter(stdout: &mut Stdout) -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(stdout, EnterAlternateScreen, cursor::Hide)?;
        Ok(Self)
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let mut stdout = io::stdout();
        let _ = execute!(stdout, cursor::Show, LeaveAlternateScreen, ResetColor);
        let _ = terminal::disable_raw_mode();
    }
}

impl App {
    fn new(debug: bool) -> Self {
        Self {
            phase: AppPhase::Puzzle,
            puzzle: new_puzzle_state(),
            email: EmailState {
                email: String::new(),
                focus: EmailFocus::Input,
                selected_button: 0,
                status: "Solve the puzzle to unlock event invite submission.".to_string(),
            },
            submitted_email: None,
            debug,
            should_quit: false,
        }
    }
}

fn main() -> io::Result<()> {
    let mut stdout = io::stdout();
    show_splash_screen(&mut stdout)?;

    let _terminal = TerminalSession::enter(&mut stdout)?;
    let mut app = App::new(debug_enabled());
    let mut needs_redraw = true;

    loop {
        if needs_redraw {
            draw_app(&mut stdout, &app)?;
            needs_redraw = false;
        }

        if app.should_quit {
            break;
        }

        if event::poll(Duration::from_millis(200))? {
            match event::read()? {
                Event::Key(key) => {
                    needs_redraw = handle_key(&mut app, key)?;
                }
                Event::Resize(_, _) => {
                    needs_redraw = true;
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn show_splash_screen(stdout: &mut Stdout) -> io::Result<()> {
    let (cols, rows) = terminal::size().unwrap_or((120, 40));
    let logo_lines: Vec<&str> = SPLASH_LOGO
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();

    execute!(
        stdout,
        Clear(ClearType::All),
        MoveTo(0, 0),
        SetBackgroundColor(Color::Black),
        cursor::Hide
    )?;

    let block_height = logo_lines.len() as u16 + 2;
    let start_y = rows.saturating_sub(block_height) / 2;

    for (offset, line) in logo_lines.iter().enumerate() {
        let x = cols.saturating_sub(line.len() as u16) / 2;
        let color = if line.contains("HACK THE WORLD") {
            Color::White
        } else {
            Color::DarkGrey
        };

        queue!(
            stdout,
            MoveTo(x, start_y + offset as u16),
            SetForegroundColor(color),
            Print(*line)
        )?;
    }

    let subheading = "ACCESS CHALLENGE INITIALIZING";
    let subheading_x = cols.saturating_sub(subheading.len() as u16) / 2;
    queue!(
        stdout,
        MoveTo(subheading_x, start_y + logo_lines.len() as u16 + 1),
        SetForegroundColor(Color::Rgb {
            r: 255,
            g: 90,
            b: 0
        }),
        SetAttribute(Attribute::Bold),
        Print(subheading),
        SetAttribute(Attribute::Reset),
        ResetColor
    )?;

    stdout.flush()?;
    thread::sleep(Duration::from_secs(3));
    execute!(
        stdout,
        Clear(ClearType::All),
        MoveTo(0, 0),
        ResetColor,
        cursor::Show
    )?;
    Ok(())
}

fn draw_app(stdout: &mut Stdout, app: &App) -> io::Result<()> {
    let (cols, rows) = terminal::size()?;
    queue!(
        stdout,
        MoveTo(0, 0),
        Clear(ClearType::All),
        SetBackgroundColor(Color::Black)
    )?;

    if cols < 78 || rows < 24 {
        draw_resize_message(stdout, cols, rows)?;
        stdout.flush()?;
        return Ok(());
    }

    let frame_width = cols.saturating_sub(6).min(108);
    let frame_x = cols.saturating_sub(frame_width) / 2;
    let header_y = 1;
    let body_y = header_y + 4;
    let body_height = rows.saturating_sub(body_y + 3);

    draw_header_bar(stdout, frame_x, header_y, frame_width, app)?;
    draw_box(
        stdout,
        frame_x,
        body_y,
        frame_width,
        body_height,
        Color::DarkGrey,
    )?;

    match app.phase {
        AppPhase::Puzzle => {
            draw_puzzle_view(stdout, frame_x, body_y, frame_width, body_height, app)?
        }
        AppPhase::Email => draw_email_view(stdout, frame_x, body_y, frame_width, body_height, app)?,
        AppPhase::Submitted => {
            draw_submitted_view(stdout, frame_x, body_y, frame_width, body_height, app)?
        }
    }

    draw_footer(stdout, frame_x, frame_width, rows, app)?;
    queue!(stdout, ResetColor, SetAttribute(Attribute::Reset))?;
    stdout.flush()?;
    Ok(())
}

fn draw_resize_message(stdout: &mut Stdout, cols: u16, rows: u16) -> io::Result<()> {
    let line_1 = "Terminal size too small for puzzle UI.";
    let line_2 = "Resize to at least 78x24.";
    let x_1 = cols.saturating_sub(line_1.len() as u16) / 2;
    let x_2 = cols.saturating_sub(line_2.len() as u16) / 2;
    let y = rows / 2;

    queue!(
        stdout,
        MoveTo(x_1, y.saturating_sub(1)),
        SetForegroundColor(Color::DarkGrey),
        Print(line_1),
        MoveTo(x_2, y + 1),
        SetForegroundColor(Color::Rgb {
            r: 255,
            g: 90,
            b: 0
        }),
        SetAttribute(Attribute::Bold),
        Print(line_2),
        SetAttribute(Attribute::Reset),
        ResetColor
    )?;
    Ok(())
}

fn draw_header_bar(stdout: &mut Stdout, x: u16, y: u16, width: u16, app: &App) -> io::Result<()> {
    let tab_label = match app.phase {
        AppPhase::Puzzle => "puzzle node",
        AppPhase::Email => "invite form",
        AppPhase::Submitted => "request sent",
    };

    let segments = vec![
        center_text("boaai", 12),
        center_text(tab_label, 16),
        center_text(
            &format!(
                "moves {}/{}",
                app.puzzle.moves_taken, app.puzzle.optimal_moves
            ),
            14,
        ),
        center_text("event access", 20),
    ];

    let content_width = segments.iter().map(String::len).sum::<usize>() + segments.len() - 1;
    if content_width as u16 + 2 > width {
        return draw_box(stdout, x, y, width, 3, Color::DarkGrey);
    }

    let mut top_border = String::from("┌");
    let mut bottom_border = String::from("└");
    for (index, segment) in segments.iter().enumerate() {
        top_border.push_str(&"─".repeat(segment.len()));
        bottom_border.push_str(&"─".repeat(segment.len()));
        if index < segments.len() - 1 {
            top_border.push('┬');
            bottom_border.push('┴');
        }
    }
    top_border.push('┐');
    bottom_border.push('┘');

    queue!(
        stdout,
        MoveTo(x, y),
        SetForegroundColor(Color::DarkGrey),
        Print(top_border),
        MoveTo(x, y + 2),
        Print(bottom_border),
        MoveTo(x, y + 1),
        Print("│")
    )?;

    let mut cursor_x = x + 1;
    for (index, segment) in segments.iter().enumerate() {
        queue!(stdout, MoveTo(cursor_x, y + 1))?;
        match index {
            0 => {
                queue!(
                    stdout,
                    SetForegroundColor(Color::White),
                    SetAttribute(Attribute::Bold),
                    Print(segment),
                    SetAttribute(Attribute::Reset),
                    SetForegroundColor(Color::DarkGrey)
                )?;
            }
            1 => {
                queue!(
                    stdout,
                    SetForegroundColor(Color::Rgb {
                        r: 255,
                        g: 90,
                        b: 0
                    }),
                    SetAttribute(Attribute::Bold),
                    Print(segment),
                    SetAttribute(Attribute::Reset),
                    SetForegroundColor(Color::DarkGrey)
                )?;
            }
            _ => {
                queue!(stdout, SetForegroundColor(Color::DarkGrey), Print(segment))?;
            }
        }

        cursor_x += segment.len() as u16;
        if index < segments.len() - 1 {
            queue!(stdout, MoveTo(cursor_x, y + 1), Print("│"))?;
            cursor_x += 1;
        }
    }

    queue!(stdout, MoveTo(cursor_x, y + 1), Print("│"), ResetColor)?;
    Ok(())
}

fn draw_box(
    stdout: &mut Stdout,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    border_color: Color,
) -> io::Result<()> {
    if width < 2 || height < 2 {
        return Ok(());
    }

    let horizontal = "─".repeat((width - 2) as usize);
    queue!(
        stdout,
        SetForegroundColor(border_color),
        MoveTo(x, y),
        Print(format!("┌{}┐", horizontal)),
        MoveTo(x, y + height - 1),
        Print(format!("└{}┘", horizontal))
    )?;

    for row in (y + 1)..(y + height - 1) {
        queue!(
            stdout,
            MoveTo(x, row),
            Print("│"),
            MoveTo(x + width - 1, row),
            Print("│")
        )?;
    }

    queue!(stdout, ResetColor)?;
    Ok(())
}

fn draw_puzzle_view(
    stdout: &mut Stdout,
    x: u16,
    body_y: u16,
    width: u16,
    body_height: u16,
    app: &App,
) -> io::Result<()> {
    let puzzle = &app.puzzle;
    let bottom = body_y + body_height - 1;
    let mut line = body_y + 1;

    queue!(
        stdout,
        MoveTo(x + 3, line),
        SetForegroundColor(Color::White),
        SetAttribute(Attribute::Bold),
        Print("LATTICE NODE // ACCESS CHALLENGE"),
        SetAttribute(Attribute::Reset),
        MoveTo(x + 3, line + 1),
        SetForegroundColor(Color::DarkGrey),
        Print("6-button custom puzzle. Use only controls below.")
    )?;

    line += 3;
    queue!(
        stdout,
        MoveTo(x + 3, line),
        SetForegroundColor(Color::DarkGrey),
        Print(format!(
            "Target   [{}]",
            render_state(TARGET_STATE).to_ascii_uppercase()
        )),
        MoveTo(x + 3, line + 1),
        Print(format!(
            "Current  [{}]",
            render_state(puzzle.current).to_ascii_uppercase()
        ))
    )?;

    let indicator_y = line + 3;
    let indicator_width = 16;
    let indicator_gap = 2;
    let indicator_span = indicator_width * INDICATOR_COUNT as u16 + indicator_gap * 3;
    let indicator_start_x = x + width.saturating_sub(indicator_span) / 2;

    if indicator_y + 2 < bottom {
        for index in 0..INDICATOR_COUNT {
            let selected = matches!(puzzle.focus, PuzzleFocus::Indicator(i) if i == index);
            let label = format!("{} {}", index + 1, puzzle.current[index].as_str());
            draw_button(
                stdout,
                indicator_start_x + index as u16 * (indicator_width + indicator_gap),
                indicator_y,
                indicator_width,
                &label,
                selected,
                puzzle.current[index].term_color(),
            )?;
        }
    }

    let action_y = indicator_y + 4;
    let action_width = 18;
    let action_gap = 2;
    let action_span = action_width * 3 + action_gap * 2;
    let action_start_x = x + width.saturating_sub(action_span) / 2;
    let action_labels = [
        "Hint",
        "Reset",
        if puzzle.show_rules {
            "Hide Rules"
        } else {
            "Show Rules"
        },
    ];

    if action_y + 2 < bottom {
        for (index, label) in action_labels.iter().enumerate() {
            let selected = matches!(puzzle.focus, PuzzleFocus::Action(i) if i == index);
            draw_button(
                stdout,
                action_start_x + index as u16 * (action_width + action_gap),
                action_y,
                action_width,
                label,
                selected,
                Color::White,
            )?;
        }
    }

    let status_y = action_y + 4;
    if status_y < bottom {
        queue!(
            stdout,
            MoveTo(x + 3, status_y),
            SetForegroundColor(Color::Rgb {
                r: 255,
                g: 90,
                b: 0
            }),
            Print(trim_to_width(
                &puzzle.status,
                width.saturating_sub(6) as usize
            ))
        )?;
    }

    if puzzle.show_rules {
        let rules = [
            "1) Pressed button advances by +2 color steps (OFF>GREEN>...>WHITE>OFF)",
            "2) Adjacent buttons (distance 1) advance by +1 step",
            "3) Distance-2 buttons move backward by 1 step",
            "4) Opposite button (distance 3) advances by +3 steps",
        ];
        let mut rules_y = status_y + 2;
        for rule in rules {
            if rules_y >= bottom {
                break;
            }
            queue!(
                stdout,
                MoveTo(x + 3, rules_y),
                SetForegroundColor(Color::DarkGrey),
                Print(trim_to_width(rule, width.saturating_sub(6) as usize))
            )?;
            rules_y += 1;
        }
    }

    if app.debug {
        queue!(
            stdout,
            MoveTo(x + 3, bottom.saturating_sub(1)),
            SetForegroundColor(Color::DarkGrey),
            Print("Debug: press F12 for instant solve")
        )?;
    }

    queue!(stdout, ResetColor)?;
    Ok(())
}

fn draw_email_view(
    stdout: &mut Stdout,
    x: u16,
    body_y: u16,
    width: u16,
    body_height: u16,
    app: &App,
) -> io::Result<()> {
    let email = &app.email;
    let bottom = body_y + body_height - 1;

    queue!(
        stdout,
        MoveTo(x + 3, body_y + 1),
        SetForegroundColor(Color::White),
        SetAttribute(Attribute::Bold),
        Print("EVENT INVITE REQUEST"),
        SetAttribute(Attribute::Reset),
        MoveTo(x + 3, body_y + 3),
        SetForegroundColor(Color::Rgb {
            r: 255,
            g: 90,
            b: 0
        }),
        Print("Warning: confirmation is final. To change it later, solve the puzzle again."),
        MoveTo(x + 3, body_y + 5),
        SetForegroundColor(Color::DarkGrey),
        Print("Email Input")
    )?;

    let field_y = body_y + 6;
    let field_width = width.saturating_sub(8).max(20);
    let field_x = x + (width.saturating_sub(field_width)) / 2;
    let is_input_selected = matches!(email.focus, EmailFocus::Input);
    let mut email_text = if email.email.is_empty() {
        "type-your-email@example.com".to_string()
    } else {
        email.email.clone()
    };
    if is_input_selected && email_text.len() < field_width.saturating_sub(4) as usize {
        email_text.push('_');
    }

    draw_button(
        stdout,
        field_x,
        field_y,
        field_width,
        &email_text,
        is_input_selected,
        if email.email.is_empty() {
            Color::DarkGrey
        } else {
            Color::White
        },
    )?;

    let button_y = field_y + 5;
    let button_width = 24;
    let button_gap = 4;
    let button_start_x = x + width.saturating_sub(button_width * 2 + button_gap) / 2;
    let buttons = ["Confirm Invite", "Solve Again"];
    for (index, label) in buttons.iter().enumerate() {
        let selected = matches!(email.focus, EmailFocus::Buttons) && email.selected_button == index;
        draw_button(
            stdout,
            button_start_x + index as u16 * (button_width + button_gap),
            button_y,
            button_width,
            label,
            selected,
            if index == 0 {
                Color::Rgb {
                    r: 255,
                    g: 90,
                    b: 0,
                }
            } else {
                Color::DarkGrey
            },
        )?;
    }

    if button_y + 4 < bottom {
        queue!(
            stdout,
            MoveTo(x + 3, button_y + 4),
            SetForegroundColor(Color::DarkGrey),
            Print("Tab switches between input and buttons. Enter activates the selected control."),
            MoveTo(x + 3, button_y + 5),
            SetForegroundColor(Color::Rgb {
                r: 255,
                g: 90,
                b: 0
            }),
            Print(trim_to_width(
                &email.status,
                width.saturating_sub(6) as usize
            ))
        )?;
    }

    queue!(stdout, ResetColor)?;
    Ok(())
}

fn draw_submitted_view(
    stdout: &mut Stdout,
    x: u16,
    body_y: u16,
    width: u16,
    _body_height: u16,
    app: &App,
) -> io::Result<()> {
    let email = app.submitted_email.as_deref().unwrap_or("unknown");
    queue!(
        stdout,
        MoveTo(x + 3, body_y + 3),
        SetForegroundColor(Color::White),
        SetAttribute(Attribute::Bold),
        Print("Invite request submitted."),
        SetAttribute(Attribute::Reset),
        MoveTo(x + 3, body_y + 5),
        SetForegroundColor(Color::DarkGrey),
        Print(trim_to_width(
            &format!("Recorded email: {email}"),
            width.saturating_sub(6) as usize
        )),
        MoveTo(x + 3, body_y + 7),
        SetForegroundColor(Color::Rgb {
            r: 255,
            g: 90,
            b: 0
        }),
        Print("Press Enter or Esc to close the SSH session.")
    )?;
    Ok(())
}

fn draw_footer(stdout: &mut Stdout, x: u16, width: u16, rows: u16, app: &App) -> io::Result<()> {
    let top = rows.saturating_sub(2);
    let bottom = rows.saturating_sub(1);
    let bar = "─".repeat(width as usize);
    let message = match app.phase {
        AppPhase::Puzzle => "Left/Right: move   Up/Down: switch row   Enter: activate   Esc: quit",
        AppPhase::Email => "Type email, Tab to buttons, Enter to activate selection, Esc to quit",
        AppPhase::Submitted => "Session complete. Press Enter or Esc to exit.",
    };

    let footer_text = trim_to_width(message, width as usize);
    let text_x = x + width.saturating_sub(footer_text.len() as u16) / 2;
    queue!(
        stdout,
        MoveTo(x, top),
        SetForegroundColor(Color::DarkGrey),
        Print(bar),
        MoveTo(text_x, bottom),
        Print(footer_text),
        ResetColor
    )?;
    Ok(())
}

fn draw_button(
    stdout: &mut Stdout,
    x: u16,
    y: u16,
    width: u16,
    label: &str,
    selected: bool,
    accent: Color,
) -> io::Result<()> {
    if width < 4 {
        return Ok(());
    }

    let inner_width = (width - 2) as usize;
    let top = format!("┌{}┐", "─".repeat(inner_width));
    let bottom = format!("└{}┘", "─".repeat(inner_width));
    let text = center_text(&trim_to_width(label, inner_width), inner_width);

    let border_color = if selected {
        Color::White
    } else {
        Color::DarkGrey
    };
    let text_color = if selected { Color::Black } else { accent };
    let fill_color = if selected { Color::Grey } else { Color::Black };

    queue!(
        stdout,
        MoveTo(x, y),
        SetForegroundColor(border_color),
        SetBackgroundColor(Color::Black),
        Print(top),
        MoveTo(x, y + 1),
        Print("│"),
        SetBackgroundColor(fill_color),
        SetForegroundColor(text_color),
        Print(text),
        SetBackgroundColor(Color::Black),
        SetForegroundColor(border_color),
        Print("│"),
        MoveTo(x, y + 2),
        Print(bottom),
        ResetColor
    )?;
    Ok(())
}

fn handle_key(app: &mut App, key: KeyEvent) -> io::Result<bool> {
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.should_quit = true;
        return Ok(true);
    }

    match app.phase {
        AppPhase::Puzzle => Ok(handle_puzzle_key(app, key)),
        AppPhase::Email => handle_email_key(app, key),
        AppPhase::Submitted => Ok(handle_submitted_key(app, key)),
    }
}

fn handle_puzzle_key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Left => {
            match app.puzzle.focus {
                PuzzleFocus::Indicator(index) => {
                    app.puzzle.focus =
                        PuzzleFocus::Indicator((index + INDICATOR_COUNT - 1) % INDICATOR_COUNT)
                }
                PuzzleFocus::Action(index) => {
                    app.puzzle.focus = PuzzleFocus::Action((index + 2) % 3)
                }
            }
            true
        }
        KeyCode::Right => {
            match app.puzzle.focus {
                PuzzleFocus::Indicator(index) => {
                    app.puzzle.focus = PuzzleFocus::Indicator((index + 1) % INDICATOR_COUNT)
                }
                PuzzleFocus::Action(index) => {
                    app.puzzle.focus = PuzzleFocus::Action((index + 1) % 3)
                }
            }
            true
        }
        KeyCode::Up | KeyCode::Down => {
            match app.puzzle.focus {
                PuzzleFocus::Indicator(index) => {
                    app.puzzle.focus = PuzzleFocus::Action((index / 2).min(2));
                }
                PuzzleFocus::Action(index) => {
                    let target = (index * 2).min(INDICATOR_COUNT - 1);
                    app.puzzle.focus = PuzzleFocus::Indicator(target);
                }
            }
            true
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            activate_puzzle_focus(app);
            true
        }
        KeyCode::F(12) if app.debug => {
            if let Some(path) = shortest_solution(app.puzzle.current, TARGET_STATE) {
                for press in &path {
                    app.puzzle.current = press_indicator(app.puzzle.current, *press);
                }
                app.puzzle.moves_taken += path.len();
                app.puzzle.status = format!("Debug solve used {} move(s).", path.len());
            } else {
                app.puzzle.status = "Debug solve did not find a valid route.".to_string();
            }

            if app.puzzle.current == TARGET_STATE {
                transition_to_email(app);
            }
            true
        }
        KeyCode::Esc => {
            app.should_quit = true;
            true
        }
        _ => false,
    }
}

fn activate_puzzle_focus(app: &mut App) {
    match app.puzzle.focus {
        PuzzleFocus::Indicator(index) => {
            app.puzzle.current = press_indicator(app.puzzle.current, index);
            app.puzzle.moves_taken += 1;
            app.puzzle.status = format!("Pressed indicator {}.", index + 1);
        }
        PuzzleFocus::Action(0) => {
            if let Some(path) = shortest_solution(app.puzzle.current, TARGET_STATE) {
                if let Some(first) = path.first() {
                    app.puzzle.status = format!("Hint: press indicator {}.", first + 1);
                } else {
                    app.puzzle.status = "State already matches target.".to_string();
                }
            } else {
                app.puzzle.status = "No hint available from this state.".to_string();
            }
        }
        PuzzleFocus::Action(1) => {
            app.puzzle.current = app.puzzle.initial;
            app.puzzle.moves_taken = 0;
            app.puzzle.status = "Puzzle reset to original generated state.".to_string();
        }
        PuzzleFocus::Action(2) => {
            app.puzzle.show_rules = !app.puzzle.show_rules;
            app.puzzle.status = if app.puzzle.show_rules {
                "Rules expanded.".to_string()
            } else {
                "Rules collapsed.".to_string()
            };
        }
        _ => {}
    }

    if app.puzzle.current == TARGET_STATE {
        transition_to_email(app);
    }
}

fn transition_to_email(app: &mut App) {
    app.phase = AppPhase::Email;
    app.email = EmailState {
        email: String::new(),
        focus: EmailFocus::Input,
        selected_button: 0,
        status: "Puzzle solved. Enter your email, then confirm invite.".to_string(),
    };
}

fn handle_email_key(app: &mut App, key: KeyEvent) -> io::Result<bool> {
    match app.email.focus {
        EmailFocus::Input => match key.code {
            KeyCode::Tab | KeyCode::Down | KeyCode::Enter => {
                app.email.focus = EmailFocus::Buttons;
                Ok(true)
            }
            KeyCode::Backspace => {
                app.email.email.pop();
                Ok(true)
            }
            KeyCode::Char(c) => {
                if is_email_char(c) && app.email.email.len() < 120 {
                    app.email.email.push(c);
                    app.email.status.clear();
                    return Ok(true);
                }
                Ok(false)
            }
            KeyCode::Esc => {
                app.should_quit = true;
                Ok(true)
            }
            _ => Ok(false),
        },
        EmailFocus::Buttons => match key.code {
            KeyCode::Tab | KeyCode::Up => {
                app.email.focus = EmailFocus::Input;
                Ok(true)
            }
            KeyCode::Left | KeyCode::Right => {
                app.email.selected_button = 1 - app.email.selected_button;
                Ok(true)
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if app.email.selected_button == 0 {
                    if !is_valid_email(&app.email.email) {
                        app.email.status =
                            "Please enter a valid email before confirming.".to_string();
                        return Ok(true);
                    }

                    store_submission(&app.email.email)?;
                    app.submitted_email = Some(app.email.email.clone());
                    app.phase = AppPhase::Submitted;
                    return Ok(true);
                }

                app.puzzle = new_puzzle_state();
                app.phase = AppPhase::Puzzle;
                Ok(true)
            }
            KeyCode::Esc => {
                app.should_quit = true;
                Ok(true)
            }
            _ => Ok(false),
        },
    }
}

fn handle_submitted_key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc | KeyCode::Enter => {
            app.should_quit = true;
            true
        }
        _ => false,
    }
}

fn new_puzzle_state() -> PuzzleState {
    let initial = START_STATE;
    let optimal_moves = shortest_solution(initial, TARGET_STATE)
        .map(|path| path.len())
        .unwrap_or(0);
    PuzzleState {
        initial,
        current: initial,
        optimal_moves,
        moves_taken: 0,
        focus: PuzzleFocus::Indicator(0),
        show_rules: false,
        status: format!(
            "All buttons start OFF. No move cap. Estimated solve depth: {}.",
            optimal_moves
        ),
    }
}

fn press_indicator(
    mut state: [NodeColor; INDICATOR_COUNT],
    index: usize,
) -> [NodeColor; INDICATOR_COUNT] {
    for target in 0..INDICATOR_COUNT {
        let clockwise = (target + INDICATOR_COUNT - index) % INDICATOR_COUNT;
        let counterclockwise = (index + INDICATOR_COUNT - target) % INDICATOR_COUNT;
        let distance = clockwise.min(counterclockwise);

        let delta = match distance {
            0 => 2, // pressed button
            1 => 1, // immediate neighbors
            2 => 5, // one step backward in color cycle
            3 => 3, // opposite button
            _ => 0,
        };

        for _ in 0..delta {
            state[target] = state[target].next();
        }
    }

    state
}

fn shortest_solution(
    start: [NodeColor; INDICATOR_COUNT],
    goal: [NodeColor; INDICATOR_COUNT],
) -> Option<Vec<usize>> {
    if start == goal {
        return Some(Vec::new());
    }

    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    let mut parent_map: HashMap<
        [NodeColor; INDICATOR_COUNT],
        ([NodeColor; INDICATOR_COUNT], usize),
    > = HashMap::new();

    queue.push_back(start);
    visited.insert(start);

    while let Some(state) = queue.pop_front() {
        for index in 0..INDICATOR_COUNT {
            let next_state = press_indicator(state, index);
            if visited.insert(next_state) {
                parent_map.insert(next_state, (state, index));
                if next_state == goal {
                    return Some(reconstruct_moves(start, goal, &parent_map));
                }
                queue.push_back(next_state);
            }
        }
    }

    None
}

fn reconstruct_moves(
    start: [NodeColor; INDICATOR_COUNT],
    goal: [NodeColor; INDICATOR_COUNT],
    parent_map: &HashMap<[NodeColor; INDICATOR_COUNT], ([NodeColor; INDICATOR_COUNT], usize)>,
) -> Vec<usize> {
    let mut cursor = goal;
    let mut path = Vec::new();

    while cursor != start {
        if let Some((previous, pressed)) = parent_map.get(&cursor) {
            path.push(*pressed);
            cursor = *previous;
        } else {
            return Vec::new();
        }
    }

    path.reverse();
    path
}

fn store_submission(email: &str) -> io::Result<()> {
    let output_path =
        env::var("BOAAI_INVITE_FILE").unwrap_or_else(|_| "invite_submissions.csv".to_string());
    let output = Path::new(&output_path);

    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let file_exists = output.exists();
    let mut file = OpenOptions::new().create(true).append(true).open(output)?;

    if !file_exists {
        writeln!(file, "submitted_unix,email")?;
    }

    let submitted_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    writeln!(file, "{submitted_unix},{email}")?;

    Ok(())
}

fn render_state(state: [NodeColor; INDICATOR_COUNT]) -> String {
    state
        .iter()
        .map(|color| color.as_str())
        .collect::<Vec<_>>()
        .join(" | ")
}

fn trim_to_width(text: &str, width: usize) -> String {
    text.chars().take(width).collect()
}

fn center_text(text: &str, width: usize) -> String {
    let clean = trim_to_width(text, width);
    let clean_len = clean.chars().count();
    if clean_len >= width {
        return clean;
    }
    let left = (width - clean_len) / 2;
    let right = width - clean_len - left;
    format!("{}{}{}", " ".repeat(left), clean, " ".repeat(right))
}

fn is_valid_email(value: &str) -> bool {
    if value.contains(' ') || value.len() < 5 {
        return false;
    }

    let mut parts = value.split('@');
    let local = parts.next().unwrap_or_default();
    let domain = parts.next().unwrap_or_default();

    if parts.next().is_some() {
        return false;
    }

    !local.is_empty() && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
}

fn is_email_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '+' | '@')
}

fn debug_enabled() -> bool {
    env::var("BOAAI_DEBUG")
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_is_all_off() {
        assert_eq!(START_STATE, [NodeColor::Off; INDICATOR_COUNT]);
    }

    #[test]
    fn shortest_solution_from_default_reaches_target() {
        let path = shortest_solution(START_STATE, TARGET_STATE).expect("path should exist");
        let mut state = START_STATE;
        for index in path {
            state = press_indicator(state, index);
        }
        assert_eq!(state, TARGET_STATE);
    }

    #[test]
    fn default_solution_sequence_matches_expected_walkthrough() {
        let path = shortest_solution(START_STATE, TARGET_STATE).expect("path should exist");
        let expected = vec![0, 1, 1, 2, 2, 2, 2, 2, 3, 4, 4, 5, 5, 5, 5, 5];
        assert_eq!(path, expected);
    }
}
