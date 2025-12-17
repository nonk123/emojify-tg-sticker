# emojify-tg-sticker

A library, a CLI tool, and a Telegram bot which slices whole pictures into custom emoji sets. This allows the picture to then be embedded into text messages as an "inline" sticker or a banner!

## Deploying a Bot

Assuming you have a Rust development environment set up, you can deploy your own bot by running the following one-liner from a Posix shell in a clone of this repository:

```sh
export TELOXIDE_TOKEN=123456789 # your bot's token from BotFather
export BOT_USERNAME=helloWorldBot # your bot's username (the one ending in `bot`)
cargo run --release --bin emojify-tg-sticker-bot
```

## CLI Tool Usage

Also provided is a simple CLI tool for slicing whole images into 100x100px Telegram emojis. Point this program to an image, and it'll produce a directory with the emoji slices in a numeric order.

Call the CLI tool as follows:

```sh
emojify-tg-sticker <input-file> [<output-directory>]
```

For example, this will output the emojis into `foo/*.png`:

```sh
emojify-tg-sticker foo.png
```

And this will output to `bar/*.png`:

```sh
emojify-tg-sticker foo.png bar
```

See `emojify-tg-sticker --help` for a complete description of the syntax.
