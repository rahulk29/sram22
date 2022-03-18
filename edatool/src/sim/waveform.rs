use crate::error::{EdaToolError, Result};
use std::{
    fmt::Display,
    io::{BufRead, BufReader, Read, Write},
};

pub enum Simulator {
    Ngspice,
    Spectre,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Waveform<'a> {
    t: &'a [f64],
    y: &'a [f64],
}

pub struct WaveformBuf {
    name: Option<String>,
    data: Vec<(f64, f64)>,
}

impl<'a> Display for Waveform<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (t, y) in self.t.iter().zip(self.y.iter()) {
            writeln!(f, "{}\t{}", *t, *y)?;
        }
        Ok(())
    }
}

impl<'a> Waveform<'a> {
    pub fn new(t: &'a [f64], y: &'a [f64]) -> Self {
        assert_eq!(t.len(), y.len());
        Self { t, y }
    }

    pub fn save<T>(&self, w: &mut T) -> Result<()>
    where
        T: Write,
    {
        writeln!(w, "# waveform saved by edatool")?;
        writeln!(w, "# one output port")?;
        writeln!(w, "# column 1: time (sec)")?;
        writeln!(w, "# column 2: value")?;
        for (t, y) in self.t.iter().zip(self.y.iter()) {
            writeln!(w, "{} {}", *t, *y)?;
        }
        Ok(())
    }
}

impl WaveformBuf {
    pub fn with_named_data(name: &str, t: &[f64], y: &[f64]) -> Self {
        Self {
            name: Some(name.to_string()),
            ..Self::with_data(t, y)
        }
    }

    pub fn with_data(t: &[f64], y: &[f64]) -> Self {
        assert_eq!(t.len(), y.len());
        let data = t
            .iter()
            .zip(y.iter())
            .map(|(t, y)| (*t, *y))
            .collect::<Vec<_>>();
        Self { name: None, data }
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn load<T>(r: &mut T) -> Result<Self>
    where
        T: Read,
    {
        let reader = BufReader::new(r);
        let mut data = Vec::new();
        for line in reader.lines() {
            let line = line?;
            println!("reading line: {}", line);
            let line = line.trim();
            let start = &line.get(..1);
            let res = match start {
                Some("#") => None,
                Some(";") => None,
                None => None,
                _ => Some(parse_waveform_line(line)?),
            };

            if let Some(point) = res {
                data.push(point);
            }
        }

        Ok(Self { name: None, data })
    }

    pub fn save<T>(&self, w: &mut T) -> Result<()>
    where
        T: Write,
    {
        writeln!(w, "# waveform saved by edatool")?;
        writeln!(w, "# one output port")?;
        writeln!(w, "# column 1: time (sec)")?;
        writeln!(w, "# column 2: value")?;
        for (t, y) in self.data.iter() {
            writeln!(w, "{} {}", *t, *y)?;
        }
        Ok(())
    }
}

impl WaveformBuf {
    #[inline]
    pub fn time(&'_ self) -> impl Iterator<Item = f64> + '_ {
        self.data.iter().map(|(t, _)| *t)
    }
    #[inline]
    pub fn values(&'_ self) -> impl Iterator<Item = f64> + '_ {
        self.data.iter().map(|(_, y)| *y)
    }
    #[inline]
    pub fn data(&'_ self) -> impl Iterator<Item = (f64, f64)> + '_ {
        self.data.iter().map(|(t, y)| (*t, *y))
    }
}

fn parse_waveform_line(line: &str) -> Result<(f64, f64)> {
    let mut split = line.split_whitespace();
    let t = split
        .next()
        .ok_or_else(|| EdaToolError::FileFormat("invalid line in waveform".to_string()))?
        .parse::<f64>()
        .map_err(|_| EdaToolError::FileFormat("unexpected value in waveform".to_string()))?;
    let y = split
        .next()
        .ok_or_else(|| EdaToolError::FileFormat("invalid line in waveform".to_string()))?
        .parse::<f64>()
        .map_err(|_| EdaToolError::FileFormat("unexpected value in waveform".to_string()))?;
    if split.next().is_some() {
        return Err(EdaToolError::FileFormat(
            "more than two values on the same line in waveform".to_string(),
        ));
    }
    Ok((t, y))
}

#[cfg(test)]
mod tests {
    use std::io::Seek;

    use super::{Waveform, WaveformBuf};

    #[test]
    fn test_save_load_waveform() -> Result<(), Box<dyn std::error::Error>> {
        let t = vec![1.0, 4.0, 8.0, 10.0, 11.0, 100.0];
        let y = vec![2.0, 3.0, 9.0, 11.0, 12.0, 245.0];
        let wav = Waveform::new(&t, &y);

        let mut file = tempfile::tempfile()?;
        wav.save(&mut file)?;
        file.seek(std::io::SeekFrom::Start(0))?;

        let wav = WaveformBuf::load(&mut file)?;
        let tn = wav.time().collect::<Vec<_>>();
        let yn = wav.values().collect::<Vec<_>>();

        assert_eq!(t, tn);
        assert_eq!(y, yn);

        Ok(())
    }
}
