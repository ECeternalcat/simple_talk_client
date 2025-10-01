# Simple Talk

A simple voice and text chat application.

## Features

- User registration and login
- Voice and text chat
- Friend system (add, remove, accept/reject requests)
- Admin panel for user and room management
- Internationalization (English and Chinese)

## Prerequisites

- [Rust](https://www.rust-lang.org/)
- [Cargo](https://crates.io/)

## Getting Started

1. **Clone the repository:**

   ```bash
   git clone https://github.com/your-username/simple_talk_client.git
   cd simple_talk_client
   ```

2. **Run the application:**

   ```bash
   cargo run
   ```

3. **Open your browser** and navigate to `http://localhost:3001` (or the port specified in `config.json`).

## Building

To build the application for release, run:

```bash
cargo build --release
```

The executable will be located in `target/release/`.

## Icon

To set the application icon on Windows, place an `icon.ico` file in the root of the project and build the application. A `build.rs` script is included to handle the icon embedding.
