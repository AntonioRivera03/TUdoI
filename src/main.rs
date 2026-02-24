use std::{cmp, io, time::Duration};

use crossterm::{
    event::{
        self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    layout::{Alignment, Constraint, Layout},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
};

#[derive(Clone, Default)]
struct TodoItem {
    text: String,
    checked: bool,
}

struct App {
    items: Vec<TodoItem>,
    selected: usize,
    cursor: usize,
    quit: bool,
}

impl App {
    fn new() -> Self {
        Self {
            items: vec![TodoItem::default()],
            selected: 0,
            cursor: 0,
            quit: false,
        }
    }

    fn run(&mut self, terminal: &mut Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>) -> io::Result<()> {
        while !self.quit {
            terminal.draw(|frame| self.draw(frame))?;

            if event::poll(Duration::from_millis(100))?
                && let Event::Key(key) = event::read()?
            {
                self.handle_key(key);
            }
        }

        Ok(())
    }

    fn draw(&self, frame: &mut ratatui::Frame) {
        let size = frame.area();
        let footer_height = 1;
        let list_height = cmp::min(self.items.len().max(1) as u16 + 2, size.height.saturating_sub(2));
        let max_text = self.items.iter().map(|i| i.text.len()).max().unwrap_or(0) as u16;
        let list_width = cmp::min(cmp::max(36, max_text + 12), size.width.saturating_sub(2));

        let vertical = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(list_height),
            Constraint::Length(footer_height),
            Constraint::Fill(1),
        ])
        .split(size);

        let horizontal = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Length(list_width),
            Constraint::Fill(1),
        ])
        .split(vertical[1]);

        let centered = horizontal[1];
        let block = Block::default().borders(Borders::ALL).title(" TODO ");
        let inner = block.inner(centered);

        frame.render_widget(block, centered);

        let lines: Vec<Line> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let marker = if i == self.selected { ">" } else { " " };
                let check = if item.checked { "x" } else { " " };
                let text = format!("{} [{}] {}", marker, check, item.text);
                if i == self.selected {
                    Line::styled(text, Style::default().fg(Color::Cyan))
                } else {
                    Line::raw(text)
                }
            })
            .collect();

        frame.render_widget(Paragraph::new(lines), inner);

        if self.selected < self.items.len() {
            let x = inner
                .x
                .saturating_add(6)
                .saturating_add(self.cursor as u16)
                .min(inner.x + inner.width.saturating_sub(1));
            let y = inner
                .y
                .saturating_add(self.selected as u16)
                .min(inner.y + inner.height.saturating_sub(1));
            frame.set_cursor_position((x, y));
        }

        let help = Paragraph::new(
            "Enter:new  Ctrl+t:toggle todo Ctrl+d:delete item  Arrows:navigate  Ctrl+q:quit",
        )
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help, vertical[2]);
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        match (key.code, key.modifiers) {
            (KeyCode::Char('q'), m) if m.contains(KeyModifiers::CONTROL) => {
                self.quit = true;
            }
            (KeyCode::Enter, m) if m.contains(KeyModifiers::SHIFT) => {
                self.toggle_selected();
            }
            (KeyCode::Char('t'), m) if m.contains(KeyModifiers::CONTROL) => {
                self.toggle_selected();
            }
            (KeyCode::Enter, _) => {
                self.insert_below();
            }
            (KeyCode::Up, _) => {
                self.move_up();
            }
            (KeyCode::Down, _) => {
                self.move_down();
            }
            (KeyCode::Left, _) => {
                self.cursor = self.cursor.saturating_sub(1);
            }
            (KeyCode::Right, _) => {
                let len = self.current_text_len();
                self.cursor = cmp::min(self.cursor + 1, len);
            }
            (KeyCode::Home, _) => {
                self.cursor = 0;
            }
            (KeyCode::End, _) => {
                self.cursor = self.current_text_len();
            }
            (KeyCode::Backspace, _) => {
                self.backspace();
            }
            (KeyCode::Delete, _) => {
                self.delete_char();
            }
            (KeyCode::Char('d'), m) if m.contains(KeyModifiers::CONTROL) => {
                self.delete_item();
            }
            (KeyCode::Char(c), m) if !m.contains(KeyModifiers::CONTROL) && !m.contains(KeyModifiers::ALT) => {
                self.insert_char(c);
            }
            _ => {}
        }
    }

    fn toggle_selected(&mut self) {
        if let Some(item) = self.items.get_mut(self.selected) {
            item.checked = !item.checked;
        }
    }

    fn insert_below(&mut self) {
        let next = self.selected + 1;
        self.items.insert(next, TodoItem::default());
        self.selected = next;
        self.cursor = 0;
    }

    fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.cursor = cmp::min(self.cursor, self.current_text_len());
        }
    }

    fn move_down(&mut self) {
        if self.selected + 1 < self.items.len() {
            self.selected += 1;
            self.cursor = cmp::min(self.cursor, self.current_text_len());
        }
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }

        if let Some(item) = self.items.get_mut(self.selected) {
            let remove_at = self.cursor - 1;
            if remove_at < item.text.len() {
                item.text.remove(remove_at);
                self.cursor -= 1;
            }
        }
    }

    fn delete_char(&mut self) {
        if let Some(item) = self.items.get_mut(self.selected)
            && self.cursor < item.text.len()
        {
            item.text.remove(self.cursor);
        }
    }

    fn delete_item(&mut self) {
        if self.items.is_empty() {
            return;
        }

        self.items.remove(self.selected);

        if self.items.is_empty() {
            self.items.push(TodoItem::default());
            self.selected = 0;
            self.cursor = 0;
            return;
        }

        if self.selected >= self.items.len() {
            self.selected = self.items.len() - 1;
        }
        self.cursor = cmp::min(self.cursor, self.current_text_len());
    }

    fn insert_char(&mut self, c: char) {
        if let Some(item) = self.items.get_mut(self.selected) {
            if self.cursor <= item.text.len() {
                item.text.insert(self.cursor, c);
                self.cursor += 1;
            }
        }
    }

    fn current_text_len(&self) -> usize {
        self.items
            .get(self.selected)
            .map(|item| item.text.len())
            .unwrap_or(0)
    }
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
        )
    )?;

    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let result = app.run(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), PopKeyboardEnhancementFlags, LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}
