use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::message::{ClientMsg, MemberInfo, ServerMsg};

// ─── Action ──────────────────────────────────────────────────────────────────

pub enum Action {
    None,
    Send(ClientMsg),
    Quit,
}

// ─── ChatLine ─────────────────────────────────────────────────────────────────

pub enum LineKind {
    Message { from: String },
    System,
    Error,
}

pub struct ChatLine {
    pub time: String,
    pub kind: LineKind,
    pub text: String,
}

impl ChatLine {
    fn now() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let h = (secs % 86400) / 3600;
        let m = (secs % 3600) / 60;
        format!("{:02}:{:02}", h, m)
    }

    pub fn system(text: impl Into<String>) -> Self {
        Self { time: Self::now(), kind: LineKind::System, text: text.into() }
    }

    pub fn error_line(text: impl Into<String>) -> Self {
        Self { time: Self::now(), kind: LineKind::Error, text: text.into() }
    }

    pub fn message(from: String, text: impl Into<String>) -> Self {
        Self { time: Self::now(), kind: LineKind::Message { from }, text: text.into() }
    }
}

// ─── App ──────────────────────────────────────────────────────────────────────

pub struct App {
    pub uuid: String,
    pub nickname: String,
    pub connected: bool,
    pub current_room: Option<String>,
    pub members: Vec<MemberInfo>,
    pub messages: Vec<ChatLine>,
    pub scroll: usize,
    pub auto_scroll: bool,
    pub input: String,
    pub cursor: usize, // byte offset
}

impl App {
    pub fn new(nickname: String, uuid: String) -> Self {
        Self {
            uuid,
            nickname,
            connected: false,
            current_room: None,
            members: Vec::new(),
            messages: Vec::new(),
            scroll: 0,
            auto_scroll: true,
            input: String::new(),
            cursor: 0,
        }
    }

    pub fn push(&mut self, line: ChatLine) {
        self.messages.push(line);
        if self.auto_scroll {
            self.scroll = self.messages.len().saturating_sub(1);
        }
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.scroll = self.scroll.saturating_sub(n);
        self.auto_scroll = false;
    }

    pub fn scroll_down(&mut self, n: usize) {
        let max = self.messages.len().saturating_sub(1);
        self.scroll = (self.scroll + n).min(max);
        if self.scroll >= max {
            self.auto_scroll = true;
        }
    }

    /// ServerMsg를 받아 상태 업데이트
    pub fn apply(&mut self, msg: ServerMsg) {
        match msg {
            ServerMsg::Connected { uuid, nickname } => {
                self.connected = true;
                let short = &uuid[..uuid.len().min(8)];
                self.push(ChatLine::system(format!("connected as {}({})", nickname, short)));
            }
            ServerMsg::Joined { room, members } => {
                let names: Vec<_> = members.iter().map(|m| m.nickname.as_str()).collect();
                self.push(ChatLine::system(format!(
                    "joined #{} | members: {}",
                    room,
                    if names.is_empty() { "(none)".to_string() } else { names.join(", ") }
                )));
                self.current_room = Some(room);
                self.members = members;
            }
            ServerMsg::Left { room } => {
                self.push(ChatLine::system(format!("left #{}", room)));
                self.current_room = None;
                self.members.clear();
            }
            ServerMsg::Message { from, text, .. } => {
                self.push(ChatLine::message(from, text));
            }
            ServerMsg::UserJoined { display, room } => {
                self.push(ChatLine::system(format!("** {} joined #{}", display, room)));
            }
            ServerMsg::UserLeft { display, room } => {
                self.push(ChatLine::system(format!("** {} left #{}", display, room)));
            }
            ServerMsg::RoomList { rooms } => {
                if rooms.is_empty() {
                    self.push(ChatLine::system("no active rooms"));
                } else {
                    self.push(ChatLine::system(format!("rooms: {}", rooms.join(", "))));
                }
            }
            ServerMsg::Error { message } => {
                self.push(ChatLine::error_line(message));
            }
        }
    }

    /// 키 이벤트 처리 → Action 반환
    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Action::Quit;
            }
            KeyCode::Char(c) => {
                self.input.insert(self.cursor, c);
                self.cursor += c.len_utf8();
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    let mut i = self.cursor - 1;
                    while !self.input.is_char_boundary(i) {
                        i -= 1;
                    }
                    self.input.drain(i..self.cursor);
                    self.cursor = i;
                }
            }
            KeyCode::Delete => {
                if self.cursor < self.input.len() {
                    let mut end = self.cursor + 1;
                    while end < self.input.len() && !self.input.is_char_boundary(end) {
                        end += 1;
                    }
                    self.input.drain(self.cursor..end);
                }
            }
            KeyCode::Left => {
                if self.cursor > 0 {
                    let mut i = self.cursor - 1;
                    while !self.input.is_char_boundary(i) {
                        i -= 1;
                    }
                    self.cursor = i;
                }
            }
            KeyCode::Right => {
                if self.cursor < self.input.len() {
                    let mut i = self.cursor + 1;
                    while i <= self.input.len() && !self.input.is_char_boundary(i) {
                        i += 1;
                    }
                    self.cursor = i;
                }
            }
            KeyCode::Home => {
                self.cursor = 0;
            }
            KeyCode::End => {
                // End key: 입력창 커서를 맨 뒤로 (스크롤 End는 Ctrl+End 또는 별도 키)
                self.cursor = self.input.len();
            }
            KeyCode::Up => self.scroll_up(1),
            KeyCode::PageUp => self.scroll_up(10),
            KeyCode::Down => self.scroll_down(1),
            KeyCode::PageDown => self.scroll_down(10),
            KeyCode::Enter => {
                let input = self.input.trim().to_string();
                self.input.clear();
                self.cursor = 0;
                return self.parse_input(input);
            }
            _ => {}
        }
        Action::None
    }

    fn parse_input(&mut self, input: String) -> Action {
        if input.is_empty() {
            return Action::None;
        }
        if let Some(cmd) = input.strip_prefix('/') {
            let mut parts = cmd.splitn(2, ' ');
            let name = parts.next().unwrap_or("");
            let arg = parts.next().map(str::trim).unwrap_or("");
            match name {
                "help" => {
                    self.push(ChatLine::system("/join <room>  — 룸 입장 (없으면 생성)"));
                    self.push(ChatLine::system("/leave        — 현재 룸 퇴장"));
                    self.push(ChatLine::system("/list         — 활성 룸 목록"));
                    self.push(ChatLine::system("/quit         — 종료"));
                    self.push(ChatLine::system("↑↓ PgUp/PgDn — 메시지 스크롤"));
                    Action::None
                }
                "join" => {
                    if arg.is_empty() {
                        self.push(ChatLine::error_line("usage: /join <room>"));
                        Action::None
                    } else {
                        Action::Send(ClientMsg::Join { room: arg.to_string() })
                    }
                }
                "leave" => Action::Send(ClientMsg::Leave),
                "list"  => Action::Send(ClientMsg::List),
                "quit"  => Action::Quit,
                other   => {
                    self.push(ChatLine::error_line(format!("unknown command: /{}", other)));
                    Action::None
                }
            }
        } else if self.current_room.is_some() {
            Action::Send(ClientMsg::Message { text: input })
        } else {
            self.push(ChatLine::system("room에 먼저 입장하세요 (/join <room>)"));
            Action::None
        }
    }
}
