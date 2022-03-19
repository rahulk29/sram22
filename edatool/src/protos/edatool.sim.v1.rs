/// Contains the results of running a single simulation.
/// A simulation has an optional name, and a list of results
/// corresponding to the different analyses performed.
///
/// Typically, a simulation will correspond to one netlist.
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SimulationData {
    /// The name of the simulation
    #[prost(string, tag="1")]
    pub name: ::prost::alloc::string::String,
    /// A list of analyses performed
    #[prost(message, repeated, tag="2")]
    pub analyses: ::prost::alloc::vec::Vec<AnalysisData>,
}
/// The parameters to be used when performing a transient analysis.
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TranParams {
    /// The simulation end time
    #[prost(double, tag="1")]
    pub tstop: f64,
    /// An initial guess for the simulation step size
    #[prost(double, tag="2")]
    pub tstep: f64,
    /// The simulation start time
    #[prost(double, tag="3")]
    pub tstart: f64,
    /// Whether or not to use initial conditions
    /// If false (default), the simulator will usually
    /// run an operating point analysis before beginning
    /// the transient simulation.
    /// If true (not supported by EdaTool at the moment),
    /// initial conditions must be manually specified.
    /// For nodes where no initial condition is explicitly given,
    /// the simulator will usually default to 0.
    #[prost(bool, tag="4")]
    pub uic: bool,
}
/// The parameters to be used when performing an AC analysis.
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AcParams {
    /// The type of frequency sweep to perform
    #[prost(enumeration="SweepMode", tag="1")]
    pub sweep_mode: i32,
    /// The number of points, number of points per decade,
    /// or number of points per octave, depending on the
    /// SweepMode used.
    #[prost(uint64, tag="2")]
    pub num: u64,
    /// The start frequency. The simulator will sweep
    /// frequencies from the start frequency to the
    /// stop frequency.
    #[prost(double, tag="3")]
    pub fstart: f64,
    /// The stop frequency. The simulator will sweep
    /// frequencies from the start frequency to the
    /// stop frequency.
    #[prost(double, tag="4")]
    pub fstop: f64,
}
/// The parameters to be used when performing a DC analysis.
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DcParams {
    /// The name of the source whose value should be swept.
    #[prost(string, tag="1")]
    pub source: ::prost::alloc::string::String,
    /// The start of the sweep range.
    #[prost(double, tag="2")]
    pub start: f64,
    /// The end of the sweep range.
    #[prost(double, tag="3")]
    pub stop: f64,
    /// The step size to use when sweeping.
    #[prost(double, tag="4")]
    pub incr: f64,
}
/// The parameters to be used when performing an operating
/// point (op) analysis.
///
/// currently empty
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct OpParams {
}
/// Indicates what type of simulation should be performed.
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AnalysisMode {
    /// Indicates what type of simulation should be performed,
    /// as well as the parameters to use for the simulation.
    #[prost(oneof="analysis_mode::Mode", tags="1, 2, 3, 4")]
    pub mode: ::core::option::Option<analysis_mode::Mode>,
}
/// Nested message and enum types in `AnalysisMode`.
pub mod analysis_mode {
    /// Indicates what type of simulation should be performed,
    /// as well as the parameters to use for the simulation.
    #[derive(serde::Serialize, serde::Deserialize)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Mode {
        #[prost(message, tag="1")]
        Tran(super::TranParams),
        #[prost(message, tag="2")]
        Ac(super::AcParams),
        #[prost(message, tag="3")]
        Dc(super::DcParams),
        #[prost(message, tag="4")]
        Op(super::OpParams),
    }
}
/// A named expression.
///
/// Corresponds to running a statement of the form
/// 
/// ```spice
/// let {name} = {expr}
/// ```
///
/// in a simulator.
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NamedExpression {
    /// The name of the expression.
    /// 
    /// The expression `expr` is evaluated, and the result
    /// is assigned to `name`.
    #[prost(string, tag="1")]
    pub name: ::prost::alloc::string::String,
    /// The actual expression to evaluate.
    /// 
    /// The expression `expr` is evaluated, and the result
    /// is assigned to `name`.
    #[prost(string, tag="2")]
    pub expr: ::prost::alloc::string::String,
}
/// An analysis to run.
///
/// An analysis consists of an `AnalysisMode`,
/// which specifies the type and parameters of the
/// simulation, as well as a list of `NamedExpression`s,
/// which specify the variables to save after the
/// simulation is done.
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Analysis {
    /// The type and parameters of the simulation.
    #[prost(message, optional, tag="1")]
    pub mode: ::core::option::Option<AnalysisMode>,
    /// A list of expressions to save/export.
    #[prost(message, repeated, tag="2")]
    pub expressions: ::prost::alloc::vec::Vec<NamedExpression>,
}
/// A container for the results of a single `Analysis`.
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AnalysisData {
    /// The type and parameters of the simulation that was run.
    #[prost(message, optional, tag="1")]
    pub mode: ::core::option::Option<AnalysisMode>,
    /// A map of values exported by the analysis. These correspond
    /// to the `NamedExpression`s specified in an `Analysis`.
    ///
    /// The keys are the names of expressions, and the values
    /// are `SimVector`s representing the results of evaluating
    /// those expressions.
    #[prost(map="string, message", tag="2")]
    pub values: ::std::collections::HashMap<::prost::alloc::string::String, SimVector>,
}
/// A vector of data produced by a simulation.
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SimVector {
    /// The (optional) name of the vector.
    #[prost(string, tag="1")]
    pub name: ::prost::alloc::string::String,
    /// The values contained in the vector.
    #[prost(oneof="sim_vector::Values", tags="2, 3")]
    pub values: ::core::option::Option<sim_vector::Values>,
}
/// Nested message and enum types in `SimVector`.
pub mod sim_vector {
    /// The values contained in the vector.
    #[derive(serde::Serialize, serde::Deserialize)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Values {
        #[prost(message, tag="2")]
        Real(super::RealVector),
        #[prost(message, tag="3")]
        Complex(super::ComplexVector),
    }
}
/// A vector of real-valued data.
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RealVector {
    /// A list of double-precision floats storing the data.
    #[prost(double, repeated, tag="1")]
    pub v: ::prost::alloc::vec::Vec<f64>,
}
/// A vector of complex-valued data.
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ComplexVector {
    /// A list of double-precision floats storing the real component of the data.
    #[prost(double, repeated, tag="1")]
    pub a: ::prost::alloc::vec::Vec<f64>,
    /// A list of double-precision floats storing the imaginary component of the data.
    #[prost(double, repeated, tag="2")]
    pub b: ::prost::alloc::vec::Vec<f64>,
}
/// Specifies how to sweep a variable
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum SweepMode {
    /// The default SweepMode, used if no other
    /// SweepMode is explicitly set.
    /// Should not be used at all.
    /// Will likely cause panics or errors if passed to EdaTool.
    Unspecified = 0,
    /// Perform a linear sweep,
    /// with a certain number of points within the sweep range.
    Linear = 1,
    /// Perform a logarithmic sweep,
    /// with a certain number of points per decade.
    Decade = 2,
    /// Perform a logarithmic sweep,
    /// with a certain number of points per octave.
    Octave = 3,
}
