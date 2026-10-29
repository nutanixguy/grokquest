# 🧙‍♂️ AI-Dventure: Interactive AI-Powered Text Adventure Game

Welcome to **AI-Dventure**! 🚀

Embark on a thrilling journey through worlds of fantasy, horror, sci-fi, or history, all narrated by a powerful AI. Your choices shape the story, your inventory grows, and your adventure is unique every time!

![Screenshot showing AI-Dventure's interactive gameplay with story text, player choices, and inventory system](screenshot.png)

## Features ✨
- **🤖 AI Storyteller:** A pluggable AI provider writes the narrative. OpenAI is the default, xAI Grok is built in, and any OpenAI-compatible API can be selected with configuration
- **🎯 Dynamic Choices:** Choose from suggested options OR type your own custom actions - the AI will seamlessly incorporate your creativity into the story
- **🎒 Persistent Game State:** Your inventory, current location, and story progress are continuously tracked and influence future events
- **🌍 Rich Themes:** Select from fantasy, horror, sci-fi, historical settings, or let the AI surprise you with a unique theme
- **💬 Natural Language Input:** Communicate with the game using natural language - no need to memorize specific commands

## How to Play 🎮
1. **Start the game** and enter your adventurer's name
2. **Choose your adventure theme** from fantasy, horror, sci-fi, historical, or let the AI surprise you
3. **Read the generated story** and decide your next move:
   - Select one of the numbered choices provided by the AI **OR**
   - Type your own creative action to take the story in an unexpected direction
4. **Continue the adventure** as the AI responds to your choices, updating your inventory and story
5. **Use special commands** like `status` to check your current state, or `quit` to exit gracefully

## Getting Started 🚀

### Prerequisites
- **Rust** (latest stable version recommended)
- An API key for the provider you choose. OpenAI keys come from [OpenAI](https://openai.com/api/). xAI keys run Grok. Any other OpenAI-compatible API works with a base URL (see below)

### Installation & Running
```bash
# Clone the repository
git clone https://github.com/your-username/ai_dventure.git
cd ai_dventure

# Run with your API key
cargo run -- --api-key <YOUR_API_KEY>

# Or set it as an environment variable
export OPENAI_API_KEY=<YOUR_API_KEY>
cargo run
```

### Choosing an AI provider

The game depends on one provider interface. `AI_PROVIDER` picks the implementation:

- `openai` (default) uses `OPENAI_API_KEY` and `gpt-4o-mini`
- `xai` uses `XAI_API_KEY`, `grok-4.7`, and `low` reasoning effort
- `compatible` calls any OpenAI-compatible server. Set `AI_BASE_URL`, `AI_API_KEY`, and `AI_MODEL`

`LLM_PROVIDER` still selects the backend when `AI_PROVIDER` is unset. `--provider` overrides both. `AI_BASE_URL` overrides the API root for any provider. `AI_MODEL` overrides the model and wins over `LLM_MODEL`.

Grok uses xAI's chat completions API. The default host is `https://api.x.ai/v1`. Set `LLM_REASONING_EFFORT=medium` when you want the narrator to think longer.

Copy the placeholder file, fill in your own key, and leave `.env` uncommitted:

```bash
cp .env.example .env
# Edit .env and set the key for the provider you selected.

set -a
source .env
set +a
cargo run
```

Or pass the environment for a single run:

```bash
# xAI Grok
AI_PROVIDER=xai XAI_API_KEY=<YOUR_XAI_API_KEY> cargo run

# The same backend through the existing LLM_PROVIDER variable
LLM_PROVIDER=xai XAI_API_KEY=<YOUR_XAI_API_KEY> LLM_REASONING_EFFORT=medium cargo run

# Any OpenAI-compatible endpoint
AI_PROVIDER=compatible \
  AI_BASE_URL=https://api.example.com/v1 \
  AI_API_KEY=<YOUR_API_KEY> \
  AI_MODEL=<MODEL_ID> \
  cargo run
```

`AI_PROVIDER=openai` is the default, so existing OpenAI setups keep working with `OPENAI_API_KEY`. Keys are read only from the environment or the matching flag.

### Add a provider

An OpenAI-compatible endpoint needs no new code. Set `AI_PROVIDER=compatible`, `AI_BASE_URL`, `AI_API_KEY`, and `AI_MODEL`.

A backend with a different request shape is three steps in `src/llm.rs`:

1. Implement the `LlmProvider` trait.
2. Match the provider name in `ProviderKind` and `provider_from_selection`.
3. Read its API key from its own environment variable in `resolve_api_key`.

## Example Gameplay
```
=== ADVENTURE BEGINS ===
You stand at the gates of a mysterious ancient castle. The wind howls through 
the twisted iron bars, and the pale moon casts eerie shadows across the 
crumbling stone walls. A faint light flickers in one of the tower windows.

Available choices:
1. Push open the heavy castle gates and enter
2. Circle around to find another entrance
3. Call out to whoever might be in the tower
4. Search the surrounding area for clues

What do you do? (type your choice or 'quit' to exit): I examine the gates for traps
```

## Why AI-Dventure is Engaging 🎭
- **🔄 Infinite Replayability:** Every playthrough is unique - the AI generates fresh content based on your choices
- **🎨 Creative Freedom:** Don't feel constrained by the given options - describe any action and watch the AI weave it into the narrative
- **📚 Rich Storytelling:** Experience detailed, atmospheric descriptions that bring each scene to life
- **🎲 Unpredictable Adventures:** The story can take unexpected turns based on your creativity and the AI's imagination

## Requirements
- Rust
- An API key for the selected provider:
  - OpenAI (`AI_PROVIDER=openai`, the default): `OPENAI_API_KEY`
  - xAI Grok (`AI_PROVIDER=xai`): `XAI_API_KEY`
  - Any OpenAI-compatible API (`AI_PROVIDER=compatible`): `AI_API_KEY`, plus `AI_BASE_URL` and `AI_MODEL`

## Command line arguments
The application accepts the following parameters:
- `--provider`: `openai` (default), `xai`, or `compatible`. When the flag is omitted, `AI_PROVIDER` is used, then `LLM_PROVIDER`
- `--api-key` / `OPENAI_API_KEY`: OpenAI key, required for the OpenAI provider
- `--xai-api-key` / `XAI_API_KEY`: xAI key, required for the xAI provider
- `--ai-api-key` / `AI_API_KEY`: key for `compatible`
- `--base-url` / `AI_BASE_URL`: API root, such as `https://api.example.com/v1`. Required for `compatible`. Optional override for OpenAI and xAI
- `--model`: model id. When the flag is omitted, `AI_MODEL` is used, then `LLM_MODEL`. Defaults to `gpt-4o-mini` for OpenAI and `grok-4.7` for xAI. Required for `compatible`. The built-in OpenAI host still accepts `gpt-3.5-turbo`, `gpt-4o`, `gpt-4o-mini`, `gpt-4-turbo`, `gpt-5`, and `openai-o4-mini`
- `--reasoning-effort` / `LLM_REASONING_EFFORT`: xAI only, `low` (default) or `medium`

Example:
```bash
# Run in development mode
cargo run

# With parameters
cargo run -- --api-key <YOUR_API_KEY> --model gpt-5

# Build a release binary
cargo build --release

# Run the release binary
./target/release/ai_dventure --api-key <YOUR_API_KEY> --model gpt-5
```

## License
[*MIT*](LICENSE)

---
Ready to shape your own legend? 🏰🦄👽
