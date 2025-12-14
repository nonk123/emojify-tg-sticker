# emojify-tg-sticker

A simple CLI tool for converting whole images into walls of consecutive emojis. Point this program to an image, and it'll produce a directory of image slices suitable for creating a custom emoji pack.

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
