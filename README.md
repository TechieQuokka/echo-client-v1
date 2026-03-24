# echo-client-v1

[echo-server-v1](https://github.com/TechieQuokka/echo-server-v1) 과 연동하는 Rust TUI 채팅 클라이언트 프로토타입.

## 기술 스택

- **Rust** (2021 edition)
- **tokio** — 비동기 런타임
- **tokio-tungstenite** — WebSocket 클라이언트
- **ratatui + crossterm** — TUI 렌더링
- **serde_json** — JSON 직렬화

## 실행 방법

```bash
# 서버를 먼저 실행한 뒤 (ws://127.0.0.1:8080)
cargo run -- <nickname>

# 서버 주소 직접 지정 시
cargo run -- <nickname> ws://127.0.0.1:8080
```

## 화면 구성

```
┌─ room: #general | members: alice, bob ────────────────┐  ← 상태바
│ [12:34] alice: 안녕하세요                              │
│ [12:34] ** bob joined #general                         │  ← 메시지창
│ [12:35] bob: 반가워요!                                 │
├────────────────────────────────────────────────────────┤
│ > _                                                    │  ← 입력창
└────────────────────────────────────────────────────────┘
```

## 명령어

| 명령어 | 설명 |
|--------|------|
| `/join <room>` | 룸 입장 (없으면 자동 생성) |
| `/leave` | 현재 룸 퇴장 |
| `/list` | 활성 룸 목록 조회 |
| `/help` | 명령어 도움말 |
| `/quit` | 종료 |
| `↑↓` / `PgUp/PgDn` | 메시지 스크롤 |
| `Ctrl+C` | 강제 종료 |

## 프로젝트 구조

```
src/
├── main.rs     — 진입점, 이벤트 루프, 채널 설정
├── message.rs  — ClientMsg / ServerMsg 타입 (serde)
├── app.rs      — App 상태 관리, 키 핸들링
├── ui.rs       — ratatui 렌더 함수
└── ws.rs       — WebSocket async 태스크
```

## 메시지 프로토콜

서버와 JSON 기반 WebSocket 프로토콜로 통신합니다.

**Client → Server**
```json
{"type": "connect",  "uuid": "...", "nickname": "alice"}
{"type": "join",     "room": "general"}
{"type": "leave"}
{"type": "message",  "text": "hello"}
{"type": "list"}
```

**Server → Client**
```json
{"type": "connected",   "uuid": "...", "nickname": "alice"}
{"type": "joined",      "room": "general", "members": [...]}
{"type": "message",     "from": "alice(1234abcd)", "room": "general", "text": "hello"}
{"type": "user_joined", "display": "bob(5678efgh)", "room": "general"}
{"type": "room_list",   "rooms": ["general", "rust"]}
```
