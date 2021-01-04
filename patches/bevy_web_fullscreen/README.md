# bevy_web_fullscreen
plugin for automatic resizing of primary bevy window to fit browser viewport

tested with [mrks-its/bevy_webgl2](https://github.com/mrk-its/bevy_webgl2) in [ostwilkens/arugio](https://github.com/ostwilkens/arugio)

currently requires specific git version of bevy:
```toml
[dependencies.bevy]
git = "https://github.com/bevyengine/bevy"
rev = "1398d7833007e85198cfd35d5fabc70b51b4db31"
default-features = false
```

### usage
`.add_plugin(FullViewportPlugin)`

### recommended html/css
```html
<meta name="viewport" content="width=device-width, user-scalable=no, minimum-scale=1.0, maximum-scale=1.0"/>
```

```css
body {
    margin: 0px;
    display: flex;
}
canvas {
    touch-action: none;
}
```