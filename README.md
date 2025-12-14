# emojify-tg-sticker

A simple CLI tool for slicing whole images into 100x100px Telegram emojis. Point this program to an image, and it'll produce a directory with the emoji slices in a numeric order.

## Usage

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
