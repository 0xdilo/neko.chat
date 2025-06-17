# neko.chat

```
        ████                      ████        
      ██    ██                  ██    ██      
      ██    ██                  ██    ██      
    ██        ██████████████████        ██    
    ██        ▓▓▓▓  ▓▓▓▓▓▓  ▓▓▓▓        ██    
    ██        ▓▓▓▓  ▓▓▓▓▓▓  ▓▓▓▓        ██    
  ██                                      ██  
  ██  ██    ████              ████    ██  ██  
  ██    ██  ████      ██      ████  ██    ██  
██    ██            ██████            ██    ██
██                                          ██
██                                          ██
██▓▓▓▓                                  ▓▓▓▓██
██▓▓▓▓                                  ▓▓▓▓██
██                                          ██

███╗   ██╗    ███████╗    ██╗  ██╗     ██████╗ 
████╗  ██║    ██╔════╝    ██║ ██╔╝    ██╔═══██╗
██╔██╗ ██║    █████╗      █████╔╝     ██║   ██║
██║╚██╗██║    ██╔══╝      ██╔═██╗     ██║   ██║
██║ ╚████║    ███████╗    ██║  ██╗    ╚██████╔╝
╚═╝  ╚═══╝    ╚══════╝    ╚═╝  ╚═╝     ╚═════╝ .chat
```

A clean chat interface for talking to various LLMs. I did this for cloneathon of t3.chat :3
It took me a couple of nights. Didn't have the time to refactor, so expect really bad code. Half vibe coded half beer coded. 
I didn't tested alot of things, expect bugs.

## What it does

- Chat with OpenAI, Anthropic, and other LLM providers
- Store your API keys securely (encrypted in SQLite)
- Organize conversations with branching support
- Branch conversations at any message to explore different paths - visualized as an interactive graph 
- Dark/light themes because we're not animals
- WebSocket real-time updates
- User authentication (local accounts + Google OAuth) (actually never tested the google auth, dont use it)
- System prompts management
- Model switching mid-conversation

## Tech stack

**Backend**: Rust + Axum + SQLite
**Frontend**: SvelteKit

Nothing fancy, just works.

## Running it

### Easy mode (recommended)
```bash
./run.sh          # Development mode
./run.sh preview  # Production build
```

### Manual mode
```bash
# Backend
cd backend
cargo run -r

# Frontend (in another terminal)
cd frontend
bun run build && bun run preview
```

### Development mode
```bash
# Backend
cd backend
cargo run

# Frontend (in another terminal) 
cd frontend
bun run dev
```

## First time setup

1. Clone this repo
2. Run `./run.sh`
3. Open http://localhost:4173 or http://localhost:5174
4. Login with `admin@admin.com` / `admin`
5. Add your API keys in Settings
6. Start chatting

The app creates a SQLite database (`nekochat.db`) automatically. Default admin account is created on first run.

## Configuration

Set these environment variables if you want:

- `DATABASE_URL` - SQLite database path (default: `nekochat.db`)
- `JWT_SECRET` - JWT signing secret (generates one if not set)
- `GOOGLE_CLIENT_ID` / `GOOGLE_CLIENT_SECRET` - For Google OAuth
- `DISABLE_ADMIN_ACCOUNT=true` - Removes the default admin account
- `SERVER_ADDR` - Server address (default: `127.0.0.1:8080`)

## API Keys

Add your API keys in the settings after logging in. They're encrypted before storage so your secrets stay secret.

Supported providers:
- OpenAI (GPT models)
- Anthropic (Claude models)  
- OpenRouter (proxy for multiple providers)

## That's it

It's a chat app. You chat with AI models. Nothing groundbreaking, just clean and functional.
