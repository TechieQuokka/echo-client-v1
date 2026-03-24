use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tungstenite::Message;

use crate::message::{ClientMsg, ServerMsg};

pub async fn run(
    mut rx: mpsc::Receiver<ClientMsg>,
    tx: mpsc::Sender<ServerMsg>,
    url: String,
) {
    let ws_stream = match connect_async(&url).await {
        Ok((ws, _)) => ws,
        Err(e) => {
            tracing::error!("연결 실패: {}", e);
            let _ = tx.send(ServerMsg::Error { message: format!("연결 실패: {}", e) }).await;
            return;
        }
    };

    tracing::info!("WebSocket 연결됨: {}", url);
    let (mut sink, mut stream) = ws_stream.split();

    loop {
        tokio::select! {
            msg = rx.recv() => match msg {
                Some(m) => {
                    match serde_json::to_string(&m) {
                        Ok(json) => {
                            if sink.send(Message::Text(json.into())).await.is_err() {
                                tracing::warn!("ws send 실패");
                                break;
                            }
                        }
                        Err(e) => tracing::error!("serialize 오류: {}", e),
                    }
                }
                None => {
                    tracing::info!("cmd 채널 닫힘, WS 종료");
                    break;
                }
            },

            frame = stream.next() => match frame {
                Some(Ok(Message::Text(t))) => {
                    match serde_json::from_str::<ServerMsg>(&t) {
                        Ok(msg) => {
                            if tx.send(msg).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => tracing::warn!("deserialize 오류: {} | raw: {}", e, t),
                    }
                }
                Some(Ok(Message::Ping(data))) => {
                    let _ = sink.send(Message::Pong(data)).await;
                }
                Some(Ok(Message::Close(_))) | None => {
                    tracing::info!("서버 연결 종료");
                    let _ = tx.send(ServerMsg::Error { message: "서버 연결이 끊겼습니다.".into() }).await;
                    break;
                }
                Some(Err(e)) => {
                    tracing::warn!("ws 오류: {}", e);
                    let _ = tx.send(ServerMsg::Error { message: format!("ws 오류: {}", e) }).await;
                    break;
                }
                _ => {}
            }
        }
    }

    let _ = sink.close().await;
    tracing::info!("ws task 종료");
}
