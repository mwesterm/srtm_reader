# SRTM parser

Reads elevation data from `.hgt` files in Rust.

## Supported resolutions

-   0.5 angle second (SRTM05) <- I made it up, not sure that's how it's called
-   1 angle second (SRTM1)
-   3 angle second (SRTM3)

## Example

```rust
use srtm::*;

let coord = (13.3255424, 56.92856);
// we get the filename, that shall include the elevation data for this `coord`
let filename = srtm::get_filename(coord);
// load the srtm, .hgt file
// NOTE: to be able to actually load it, you'll need the actual file
let tile = srtm::Tile::from_file(filename).unwrap();
// and finally, retrieve our elevation data
let elevation = tile.get(coord);
```

## _NOTE_

a great source of srtm, `.hgt` files is [sonny's collection](https://sonny.4lima.de/)
