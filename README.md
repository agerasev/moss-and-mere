# Moss & Mere — web release

This branch contains only the deployable web build. Source code lives on `master`.

- [Play the game](https://agerasev.github.io/moss-and-mere/)
- [Example screenshot](https://agerasev.github.io/moss-and-mere/preview.png)
- Source commit: `a9f89d9a5808916f278880007448c3bc20e69b33`

![Emberwatch Ruins](preview.png)

Configure GitHub Pages to deploy from `gh-pages`, folder `/(root)`.

Build from the source checkout with its neighboring wgame checkout:

```sh
trunk build --release --locked --no-default-features --features web --public-url ./
```

Copy the generated files to this branch's root. Include `.nojekyll`, `LICENSE`,
`FONT-LICENSE.txt`, and the stable `preview.png` image. Generated assets do not
belong on the source branch. The font license covers the fonts embedded in WASM.
