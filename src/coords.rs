/// coordinates
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Default)]
pub struct Coord {
    /// latitude: north-south
    pub lat: f64,
    /// longitude: east-west
    pub lon: f64,
}

impl Coord {
    pub fn opt_new(lat: impl Into<f64>, lon: impl Into<f64>) -> Option<Self> {
        let lat = lat.into();
        let lon = lon.into();
        if (-90. ..=90.).contains(&lat) && (-180. ..=180.).contains(&lon) {
            Some(Self { lat, lon })
        } else {
            None
        }
    }
    pub fn new(lat: impl Into<f64>, lon: impl Into<f64>) -> Self {
        Self::opt_new(lat, lon).expect("latitude must be between -90 and 90 degrees, longitude must be between -180 and 180 degrees")
    }
    pub fn with_lat(self, lat: impl Into<f64>) -> Self {
        Self::new(lat, self.lon)
    }
    pub fn with_lon(self, lon: impl Into<f64>) -> Self {
        Self::new(self.lat, lon)
    }
    pub fn add_to_lat(self, lat: impl Into<f64>) -> Self {
        self.with_lat(self.lat + lat.into())
    }
    pub fn add_to_lon(self, lon: impl Into<f64>) -> Self {
        self.with_lon(self.lon + lon.into())
    }

    /// truncate both latitude and longitude
    /// use no_std compatible `to_int_unchecked` method
    pub fn trunc(&self) -> (i8, i16) {
        let lat_trunc = unsafe { self.lat.to_int_unchecked::<i8>() };
        let lon_trunc = unsafe { self.lon.to_int_unchecked::<i16>() };
        (lat_trunc, lon_trunc)
    }

    /// get the name of the file, which shall include this `coord`s elevation
    ///
    /// # Usage
    ///
    /// ```rust
    /// // the `coord`inate, we want the elevation for
    /// let coord = srtm_reader::Coord::new(87.235, 10.4234423);
    /// let filename = coord.get_filename();
    /// assert_eq!(filename, "N87E010.hgt");
    /// ```
    pub fn get_filename(self) -> String {
        let lat_ch = if self.lat >= 0. { 'N' } else { 'S' };
        let lon_ch = if self.lon >= 0. { 'E' } else { 'W' };
        let (lat, lon) = self.trunc();
        let (lat, lon) = (lat.abs(), lon.abs());
        format!(
            "{lat_ch}{}{lat}{lon_ch}{}{lon}.hgt",
            if lat < 10 { "0" } else { "" },
            if lon < 10 {
                "00"
            } else if lon < 100 {
                "0"
            } else {
                ""
            }
        )
    }
}

impl<F1: Into<f64>, F2: Into<f64>> From<(F1, F2)> for Coord {
    fn from(value: (F1, F2)) -> Self {
        let (lat, lon) = (value.0.into(), value.1.into());
        Coord { lat, lon }
    }
}
