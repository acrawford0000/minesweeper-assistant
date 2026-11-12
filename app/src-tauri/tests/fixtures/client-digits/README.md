These small regression crops render the official minesweeper.online tile artwork:
`https://minesweeper.online/img/skins/{hd,hdd}/type{6,7,8}.svg?v=16`.
`hd` is light mode; `hdd` is dark mode. Chromium rendered the crops at their native
pixel sizes. Reference models use 16, 24, 30, and 40 pixels; 20, 28, and 36 pixels
are held-out scales for regression tests. Rust applies the same foreground
extraction and normalization to both reference crops and screenshot cells.
