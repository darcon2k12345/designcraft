# Patronus Design app icon

The Patronus shield with a "P", in green for Patronus Design (red for Patronus PDF, blue for
Patronus Photo, silver for the Patronus Launcher), on a dark rounded tile so that it reads on
light and dark backgrounds.

`source/patronus-design-shield.png` is the transparent shield master. Every other file in this
directory is generated from it by `packaging/make-icons.py` (needs Pillow):

```
python3 packaging/make-icons.py assets/app-icon/source/patronus-design-shield.png assets/app-icon patronusdesign com.patronus.patronusdesign
```

That writes `patronusdesign-1024.png`, `patronusdesign-macos-512.png`, `patronusdesign.ico`,
`patronusdesign.icns`, the two SVGs (which embed the 512 px PNG) and `hicolor/`. The outputs are
committed, so builds and packaging never need Pillow. The master was cut from the logo supplied by
the project owner, so its resolution is limited: a vector original would give sharper large sizes.
