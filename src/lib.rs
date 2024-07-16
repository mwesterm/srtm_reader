//! srtm data: `.hgt` file parser
//!
//! ```rust
//! use srtm::*;
//! use std::path::PathBuf;
//!
//! let coord = (29.3255424, -14.92856);
//! // we get the filename, that shall include the elevation data for this `coord`
//! let filename = srtm::get_filename(coord);
//! // in this case, the filename will be:
//! assert_eq!(filename, "N29W014.hgt");
//! // where all the srtm data: `.hgt` files are stored
//! let elev_data_dir = PathBuf::from(env!("ELEV_DATA_DIR"));
//! // where the data is located
//! let filepath = elev_data_dir.join(filename);
//! // load the srtm, .hgt file
//! let tile = srtm::Tile::from_file(filepath).unwrap();
//! // and finally, retrieve our elevation data
//! let elevation = tile.get(coord);
//! ```

use byteorder::{BigEndian, ReadBytesExt};
use std::fs::{self, File};
use std::io::{self, BufReader, Read};
use std::path::Path;

const EXTENT: usize = 3600;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Resolution {
    SRTM05,
    SRTM1,
    SRTM3,
}

impl Resolution {
    pub const fn extent(&self) -> usize {
        match self {
            Resolution::SRTM05 => EXTENT * 2 + 1,
            Resolution::SRTM1 => EXTENT + 1,
            Resolution::SRTM3 => EXTENT / 3 + 1,
        }
    }
    pub const fn total_size(&self) -> usize {
        self.extent().pow(2)
    }
}

#[derive(Debug)]
pub struct Tile {
    pub latitude: i32,
    pub longitude: i32,
    pub resolution: Resolution,
    pub data: Vec<i16>,
}

#[derive(Debug)]
pub enum Error {
    ParseLatLong,
    Filesize,
    Read,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Coord {
    lat: f64,
    lon: f64,
}
impl Coord {
    pub fn new<F1: Into<f64>, F2: Into<f64>>(lat: F1, lon: F2) -> Self {
        Self {
            lat: lat.into(),
            lon: lon.into(),
        }
    }

    /// truncate both latitude and longitude
    pub fn trunc(&self) -> (i32, i32) {
        (self.lat.trunc() as i32, self.lon.trunc() as i32)
    }
}
impl<F1: Into<f64>, F2: Into<f64>> From<(F1, F2)> for Coord {
    fn from(value: (F1, F2)) -> Self {
        let (lat, lon) = (value.0.into(), value.1.into());
        Coord { lat, lon }
    }
}
impl Tile {
    fn new_empty(lat: i32, lon: i32, res: Resolution) -> Tile {
        Tile {
            latitude: lat,
            longitude: lon,
            resolution: res,
            data: Vec::new(),
        }
    }

    /// read an srtm: `.hgt` file, and create a [`Tile`] if possible
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Tile, Error> {
        let (lat, lon) = get_lat_long(&path)?;
        let res = get_resolution(&path).ok_or(Error::Filesize)?;
        // eprintln!("resolution: {res:?}");
        let file = File::open(&path).map_err(|_| Error::Read)?;
        // eprintln!("file: {file:?}");
        let reader = BufReader::new(file);
        let mut tile = Tile::new_empty(lat, lon, res);
        tile.data = parse(reader, tile.resolution).map_err(|e| {
            eprintln!("parse error: {e:#?}");
            Error::Read
        })?;
        Ok(tile)
    }

    /// the maximum height that this [`Tile`] contains
    pub fn max_height(&self) -> i16 {
        *(self.data.iter().max().unwrap())
    }
    /// get lower-left corner's latitude and longitude
    /// it's needed for [`Tile::get_offset()`]
    fn get_origin(&self, coord: Coord) -> Coord {
        let lat = coord.lat.trunc() + 1.; // The latitude of the lower-left corner of the tile
        let lon = coord.lon.trunc(); // The longitude of the lower-left corner of the tile
        Coord { lat, lon }
    }
    /// calculate where this `coord` is located in this [`Tile`]
    fn get_offset(&self, coord: Coord) -> (usize, usize) {
        let origin = self.get_origin(coord);
        // eprintln!("origin: ({}, {})", origin.0, origin.1);
        let extent = self.resolution.extent() as f64;

        let row = ((origin.lat - coord.lat) * extent) as usize;
        let col = ((coord.lon - origin.lon) * extent) as usize;
        (row, col)
    }

    /// get the elevation of this `coord` from this [`Tile`]
    ///
    /// # Panics
    /// If this [`Tile`] doesn't contain `coord`'s elevation
    /// *NOTE*: shouldn't happen if [`get_filename()`] was used
    pub fn get<C: Into<Coord>>(&self, coord: C) -> i16 {
        let coord: Coord = coord.into();
        let offset = self.get_offset(coord);
        let lat = coord.lat.trunc() as i32;
        let lon = coord.lon.trunc() as i32;
        assert!(
            self.latitude <= lat,
            "hgt lat: {}, coord lat: {lat}",
            self.latitude
        );
        assert!(
            self.longitude <= lon,
            "hgt lon: {}, coord lon: {lon}",
            self.longitude
        );
        // eprintln!("offset: ({}, {})", offset.1, offset.0);
        self.get_at_offset(offset.1, offset.0)
    }

    fn get_at_offset(&self, x: usize, y: usize) -> i16 {
        self.data[self.idx(x, y)]
    }

    fn idx(&self, x: usize, y: usize) -> usize {
        assert!(x < self.resolution.extent() && y < self.resolution.extent());
        y * self.resolution.extent() + x
    }
}

/// guess the resolution of the file at `path`
fn get_resolution<P: AsRef<Path>>(path: P) -> Option<Resolution> {
    let from_metadata = |m: fs::Metadata| {
        let len = m.len() as usize;
        // eprintln!("len: {len}");
        if len == Resolution::SRTM05.total_size() * 2 {
            Some(Resolution::SRTM05)
        } else if len == Resolution::SRTM1.total_size() * 2 {
            Some(Resolution::SRTM1)
        } else if len == Resolution::SRTM3.total_size() * 2 {
            Some(Resolution::SRTM3)
        } else {
            eprintln!("unknown filesize: {}", len);
            None
        }
    };
    fs::metadata(path)
        .inspect_err(|e| eprintln!("error: {e:#?}"))
        .ok()
        .and_then(from_metadata)
}

// FIXME Better error handling.
fn get_lat_long<P: AsRef<Path>>(path: P) -> Result<(i32, i32), Error> {
    let stem = path.as_ref().file_stem().ok_or(Error::ParseLatLong)?;
    let desc = stem.to_str().ok_or(Error::ParseLatLong)?;
    if desc.len() != 7 {
        return Err(Error::ParseLatLong);
    }

    let get_char = |n| desc.chars().nth(n).ok_or(Error::ParseLatLong);
    let lat_sign = if get_char(0)? == 'N' { 1 } else { -1 };
    let lat: i32 = desc[1..3].parse().map_err(|_| Error::ParseLatLong)?;

    let lon_sign = if get_char(3)? == 'E' { 1 } else { -1 };
    let lon: i32 = desc[4..7].parse().map_err(|_| Error::ParseLatLong)?;
    Ok((lat_sign * lat, lon_sign * lon))
}
/// get the name of the file, which shall include this `coord`s elevation
///
/// # Usage
/// ```rust
/// // the `coord`inate, whe want the elevation for
/// let coord = (87.235, 10.4234423);
/// // this convenient function gives us the filename for
/// // any `coord`inate, that `impl Into<srtm::Coord>`
/// // which is done for this tuple
/// let filename = srtm::get_filename(coord);
/// assert_eq!(filename, "N87E010.hgt");
/// ```
pub fn get_filename<C: Into<Coord>>(coord: C) -> String {
    let coord: Coord = coord.into();
    let lat_ch = if coord.lat >= 0. { 'N' } else { 'S' };
    let lon_ch = if coord.lon >= 0. { 'E' } else { 'W' };
    let lat = (coord.lat.trunc() as i32).abs();
    let lon = (coord.lon.trunc() as i32).abs();
    format!(
        "{lat_ch}{lat}{lon_ch}{}{lon}.hgt",
        if lon == 0 {
            "00"
        } else if lon < 100 {
            "0"
        } else {
            ""
        }
    )
}
fn parse<R: Read>(reader: R, res: Resolution) -> io::Result<Vec<i16>> {
    let mut reader = reader;
    let mut data = Vec::new();
    // eprintln!("total size: {}", res.total_size());
    for _ in 0..res.total_size() {
        // eprint!("{i} ");
        let h = reader.read_i16::<BigEndian>()?;
        data.push(h);
    }
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn parse_latitute_and_longitude() {
        let ne = Path::new("N35E138.hgt");
        assert_eq!(get_lat_long(ne).unwrap(), (35, 138));

        let nw = Path::new("N35W138.hgt");
        assert_eq!(get_lat_long(nw).unwrap(), (35, -138));

        let se = Path::new("S35E138.hgt");
        assert_eq!(get_lat_long(se).unwrap(), (-35, 138));

        let sw = Path::new("S35W138.hgt");
        assert_eq!(get_lat_long(sw).unwrap(), (-35, -138));
    }
    #[test]
    fn total_file_sizes() {
        assert_eq!(103_708_802 / 2, Resolution::SRTM05.total_size());
        assert_eq!(25_934_402 / 2, Resolution::SRTM1.total_size());
        assert_eq!(2_884_802 / 2, Resolution::SRTM3.total_size());
    }
    #[test]
    fn extents() {
        assert_eq!(7201, Resolution::SRTM05.extent());
        assert_eq!(3601, Resolution::SRTM1.extent());
        assert_eq!(1201, Resolution::SRTM3.extent());
    }
}
