mod app;
mod message;
mod ui;
mod ws;

use std::{env, io};

use crossterm::{
    event::{Event, EventStream},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures_util::StreamExt;
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc;
use uuid::Uuid;

use app::{Action, App, ChatLine};
use message::ClientMsg;

const DEFAULT_SERVER: &str = "ws://127.0.0.1:8080";

#[tokio::main]
async fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <nickname> [server_url]", args[0]);
        std::process::exit(1);
    }
    let nickname = args[1].clone();
    let server_url = args.get(2).cloned().unwrap_or_else(|| DEFAULT_SERVER.to_string());

    // tracing → 파일 (TUI가 터미널 소유하므로 stdout/stderr 출력 금지)
    std::fs::create_dir_all("logs").ok();
    let appender = tracing_appender::rolling::daily("logs", "echo-client.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(appender);
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .init();

    let uuid = Uuid::new_v4().to_string();

    // 채널 생성
    let (tx_cmd, rx_cmd) = mpsc::channel::<ClientMsg>(64);
    let (tx_event, mut rx_event) = mpsc::channel::<message::ServerMsg>(256);

    let mut app = App::new(nickname, uuid);

    // WS 태스크 스폰
    tokio::spawn(ws::run(rx_cmd, tx_event, server_url));

    // Connect 메시지 즉시 전송
    let _ = tx_cmd
        .send(ClientMsg::Connect { uuid: app.uuid.clone(), nickname: app.nickname.clone() })
        .await;

    // 터미널 설정
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut events = EventStream::new();
    let mut ws_done = false;

    // 이벤트 루프
    'main: loop {
        terminal.draw(|f| ui::render(f, &app))?;

        tokio::select! {
            maybe_event = events.next() => {
                match maybe_event {
                    Some(Ok(Event::Key(key))) => {
                        match app.handle_key(key) {
                            Action::Quit => break 'main,
                            Action::Send(msg) => {
                                if tx_cmd.send(msg).await.is_err() {
                                    break 'main;
                                }
                            }
                            Action::None => {}
                        }
                    }
                    Some(Ok(Event::Resize(_, _))) => {} // 다음 루프에서 재렌더
                    Some(Err(e)) => {
                        tracing::error!("event 오류: {}", e);
                        break 'main;
                    }
                    None => break 'main,
                    _ => {}
                }
            }

            maybe_msg = rx_event.recv(), if !ws_done => {
                match maybe_msg {
                    Some(msg) => app.apply(msg),
                    None => {
                        // WS 태스크 종료 — 더 이상 이 arm을 폴링하지 않음
                        ws_done = true;
                        app.connected = false;
                        app.push(ChatLine::system("서버와의 연결이 끊겼습니다. Ctrl+C로 종료하세요."));
                    }
                }
            }
        }
    }

    // 터미널 복원
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(())
}
