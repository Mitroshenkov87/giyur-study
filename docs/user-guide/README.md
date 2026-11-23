# User guide sources

HTML and CSS for the Shorashim user guide PDF, published as
[`docs/Shorashim-User-Guide.pdf`](../Shorashim-User-Guide.pdf).

The printed figures are taken from `docs/screenshots` (WebP). The build
converts them only in a temporary folder; this repository does not add
PNG files.

Rebuild (WeasyPrint and Pillow):

```sh
python3 tools/make_user_guide.py
```
