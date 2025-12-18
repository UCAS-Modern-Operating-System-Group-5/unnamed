# Development

## Prerequisite

Download [Noto Sans CJK font](https://github.com/notofonts/noto-cjk/tree/main/Sans#downloading-noto-sans-cjk) (Region-specific Subset OTFs > China (中国)). The [download link](https://github.com/googlefonts/noto-cjk/releases/download/Sans2.004/18_NotoSansSC.zip). Move the `NotoSansSC-Regular.otf` file to `./assets/NotoSansSC-Regular.otf`.

## Profiling Tips

### Monitor Resource Occupation

```
cargo run -p gui # At the project root
htop -p $(pgrep gui) # In another terminal
```
