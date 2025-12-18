# Development

## Prerequisite

Download [Maple Font](https://github.com/subframe7536/maple-font/releases/latest) CN
No-Ligature font, and then move the `MapleMonoNL-CN-Regular.ttf` file to `./assets/MapleMonoNL-CN-Regular.ttf`.


## Profiling Tips

### Monitor Resource Occupation

```
cargo run -p gui # At the project root
htop -p $(pgrep gui) # In another terminal
```
