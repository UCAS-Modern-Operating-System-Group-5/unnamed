# Development

## Prerequisite

Download [Noto Sans CJK font](https://github.com/notofonts/noto-cjk/tree/main/Sans#downloading-noto-sans-cjk) (In the section: [Language-specific OTFs](https://github.com/notofonts/noto-cjk/tree/main/Sans#language-specific-otfs) -> Simplified Chinese (and its mono version)).

You need to move `NotoSansCJKsc-Regular.otf` and `NotoSansMonoCJKsc-Regular.otf` files to `./assets` directory.

## Profiling Tips

### Profile with [Puffin](https://github.com/EmbarkStudios/puffin)

```
cargo run -p gui -F profile-with-puffin -- --profile
```

### Monitor Resource Occupation

```
cargo run -p gui # At the project root
htop -p $(pgrep gui) # In another terminal
```


## TODO

- [] Preview Panel
- [] Set Application Logo. https://github.com/emilk/egui/discussions/5356
