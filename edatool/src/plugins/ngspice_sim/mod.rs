use crate::error::{EdaToolError, Result};
use crate::protos::sim::analysis_mode::Mode;
use crate::protos::sim::sim_vector::Values;
use crate::protos::sim::{
    Analysis, AnalysisData, AnalysisMode, ComplexVector, RealVector, SimVector, SimulationData,
    SweepMode,
};
use crate::sim::testbench::{NetlistSource, Testbench};
use std::collections::HashMap;
use std::fs::{self, read_to_string, File};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

#[cfg(test)]
mod tests;

pub struct Ngspice {
    cwd: Option<PathBuf>,
    tb: Testbench,
    analyses: Vec<Analysis>,
    control: String,
}

impl Ngspice {
    pub fn with_tb(tb: Testbench) -> Self {
        Self {
            cwd: None,
            tb,
            analyses: vec![],
            control: "".to_string(),
        }
    }

    #[inline]
    pub fn cwd(&mut self, cwd: PathBuf) -> &mut Self {
        self.cwd = Some(cwd);
        self
    }

    pub fn add_analysis(&mut self, a: Analysis) -> Result<&mut Self> {
        let mode = a.mode.as_ref().ok_or_else(EdaToolError::no_analysis_mode)?;
        let spice_line = self.prepare_spice_analysis(mode)?;

        self.control.push_str(&spice_line);

        let out_file = get_out_file(self.analyses.len());
        if !a.expressions.is_empty() {
            let mut wrdata = format!("wrdata {}", out_file);
            for expr in a.expressions.iter() {
                self.control
                    .push_str(&format!("let {} = {}\n", &expr.name, &expr.expr));
                wrdata.push_str(&format!(" {}", &expr.name));
            }
            self.control.push_str(&wrdata);
            self.control.push('\n');
        }

        self.analyses.push(a);

        Ok(self)
    }

    pub fn run(self) -> Result<SimulationData> {
        let cwd = if let Some(cwd) = self.cwd.as_ref() {
            cwd.to_owned()
        } else {
            std::env::current_dir()?
        };

        fs::create_dir_all(&cwd)?;

        let path = self.write_tb_to_file(&cwd, &self.tb)?;

        let mut cmd = Command::new("ngspice");
        cmd.arg("-b");
        cmd.arg(path);

        cmd.current_dir(&cwd);

        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut child = cmd.spawn()?;
        child.wait()?;

        let analyses = self
            .analyses
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let out_file_path = cwd.join(get_out_file(i));
                println!("reading from {:?}", &out_file_path);
                let data = read_analysis_data(a, out_file_path)?;
                Ok(data)
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(SimulationData {
            name: "ngspice_sim".to_string(),
            analyses,
        })
    }

    fn write_tb_to_file(&self, cwd: impl AsRef<Path>, tb: &Testbench) -> Result<PathBuf> {
        let path = cwd.as_ref().join("ztop.spice");
        let mut f = File::create(&path)?;
        self.write_tb(&mut f, tb)?;
        f.flush()?;

        // Write all waveforms into files
        for wav in tb.waveforms() {
            let name = wav.name().unwrap();
            let path = cwd.as_ref().join(name);
            let mut f = File::create(&path)?;
            wav.save(&mut f)?;
            f.flush()?;
        }

        Ok(path)
    }

    fn write_tb<T>(&self, dst: &mut T, tb: &Testbench) -> Result<()>
    where
        T: Write,
    {
        let title = tb.name().unwrap_or("Netlist generated by Sram22");

        writeln!(dst, "* {}", title)?;

        for include in tb.includes() {
            writeln!(dst, ".include {}", include.to_str().unwrap())?;
        }

        for lib in tb.libs() {
            writeln!(
                dst,
                ".lib {} {}",
                lib.path.to_str().unwrap(),
                lib.name.as_deref().unwrap_or("")
            )?;
        }

        match tb.source() {
            NetlistSource::Str(s) => writeln!(dst, "{}", s)?,
            NetlistSource::File(p) => {
                let s = read_to_string(p)?;
                writeln!(dst, "{}", s)?;
            }
        }

        writeln!(dst, ".control")?;
        writeln!(dst, "{}", self.control)?;
        writeln!(dst, ".endc")?;
        writeln!(dst, ".end")?;
        Ok(())
    }

    fn prepare_spice_analysis(&self, m: &AnalysisMode) -> Result<String> {
        let m = m.mode.as_ref().ok_or_else(EdaToolError::no_analysis_mode)?;
        Ok(match m {
            Mode::Op(_) => "op\n".to_string(),
            Mode::Tran(ref m) => {
                let uic = if m.uic { "uic" } else { "" };
                format!("tran {}s {}s {}s {}\n", m.tstep, m.tstop, m.tstart, uic)
            }
            Mode::Dc(ref m) => {
                format!("dc {} {} {} {}\n", m.source, m.start, m.stop, m.incr)
            }
            Mode::Ac(ref m) => {
                let sweep_mode = SweepMode::from_i32(m.sweep_mode).unwrap();
                format!("ac {} {} {} {}", sweep_mode, m.num, m.fstart, m.fstop)
            }
        })
    }
}

fn get_out_file(id: usize) -> String {
    format!("_ngspice_out_{}.m", id)
}

fn read_analysis_data(a: &Analysis, out_file: impl AsRef<Path>) -> Result<AnalysisData> {
    let f = File::open(out_file)?;
    let reader = BufReader::new(f);
    let data: Vec<Vec<f64>> = reader
        .lines()
        .map(|line| {
            let line = line?;
            let row = line
                .trim()
                .split_whitespace()
                .map(|s| s.parse::<f64>())
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|_| {
                    EdaToolError::FileFormat(
                        "invalid output data format from ngspice simulation".to_string(),
                    )
                })?;
            Ok::<Vec<f64>, EdaToolError>(row)
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;

    // Results will be complex numbers if running AC analysis
    let mode = a
        .mode
        .as_ref()
        .ok_or_else(EdaToolError::no_analysis_mode)?
        .mode
        .as_ref()
        .ok_or_else(EdaToolError::no_analysis_mode)?;
    let complex = matches!(mode, Mode::Ac(_));

    // sweep var is 1st col
    let mut sweep_var = Vec::new();
    let mut results = HashMap::new();

    for row in data.into_iter() {
        sweep_var.push(row[0]);
        let mut counter = 1;
        for expr in a.expressions.iter() {
            if !results.contains_key(&expr.name) {
                if complex {
                    results.insert(
                        expr.name.clone(),
                        SimVector {
                            name: expr.name.clone(),
                            values: Some(Values::Complex(ComplexVector::default())),
                        },
                    );
                } else {
                    results.insert(
                        expr.name.clone(),
                        SimVector {
                            name: expr.name.clone(),
                            values: Some(Values::Real(RealVector::default())),
                        },
                    );
                }
            }
            if let Some(entry) = results.get_mut(&expr.name) {
                match entry.values.as_mut().unwrap() {
                    Values::Complex(ref mut v) => {
                        v.a.push(row[counter]);
                        v.b.push(row[counter + 1]);
                        counter += 3;
                    }
                    Values::Real(ref mut v) => {
                        v.v.push(row[counter]);
                        counter += 2;
                    }
                }
            } else {
                unreachable!();
            }
        }
    }

    if results.contains_key("sweep_var") {
        return Err(EdaToolError::InvalidArgs(
            "cannot have variable named `sweep_var` in list of expressions to save".to_string(),
        ));
    }

    results.insert(
        "sweep_var".to_string(),
        SimVector {
            name: "sweep_var".to_string(),
            values: Some(Values::Real(RealVector { v: sweep_var })),
        },
    );

    Ok(AnalysisData {
        mode: Some(a.mode.clone().unwrap()),
        values: results,
    })
}
