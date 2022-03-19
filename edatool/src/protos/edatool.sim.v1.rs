#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SimulationData {
    #[prost(string, tag="1")]
    pub name: ::prost::alloc::string::String,
    #[prost(message, repeated, tag="2")]
    pub analyses: ::prost::alloc::vec::Vec<AnalysisData>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TranParams {
    #[prost(double, tag="1")]
    pub tstop: f64,
    #[prost(double, tag="2")]
    pub tstep: f64,
    #[prost(double, tag="3")]
    pub tstart: f64,
    #[prost(bool, tag="4")]
    pub uic: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AcParams {
    #[prost(enumeration="SweepMode", tag="1")]
    pub sweep_mode: i32,
    #[prost(uint64, tag="2")]
    pub num: u64,
    #[prost(double, tag="3")]
    pub fstart: f64,
    #[prost(double, tag="4")]
    pub fstop: f64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DcParams {
    #[prost(string, tag="1")]
    pub source: ::prost::alloc::string::String,
    #[prost(double, tag="2")]
    pub start: f64,
    #[prost(double, tag="3")]
    pub stop: f64,
    #[prost(double, tag="4")]
    pub incr: f64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct OpParams {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AnalysisMode {
    #[prost(oneof="analysis_mode::Mode", tags="1, 2, 3, 4")]
    pub mode: ::core::option::Option<analysis_mode::Mode>,
}
/// Nested message and enum types in `AnalysisMode`.
pub mod analysis_mode {
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
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NamedExpression {
    #[prost(string, tag="1")]
    pub name: ::prost::alloc::string::String,
    #[prost(string, tag="2")]
    pub expr: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Analysis {
    #[prost(message, optional, tag="1")]
    pub mode: ::core::option::Option<AnalysisMode>,
    #[prost(message, repeated, tag="2")]
    pub expressions: ::prost::alloc::vec::Vec<NamedExpression>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AnalysisData {
    #[prost(message, optional, tag="1")]
    pub mode: ::core::option::Option<AnalysisMode>,
    #[prost(map="string, message", tag="2")]
    pub values: ::std::collections::HashMap<::prost::alloc::string::String, SimVector>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SimVector {
    #[prost(string, tag="1")]
    pub name: ::prost::alloc::string::String,
    #[prost(oneof="sim_vector::Values", tags="2, 3")]
    pub values: ::core::option::Option<sim_vector::Values>,
}
/// Nested message and enum types in `SimVector`.
pub mod sim_vector {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Values {
        #[prost(message, tag="2")]
        Real(super::RealVector),
        #[prost(message, tag="3")]
        Complex(super::ComplexVector),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RealVector {
    #[prost(double, repeated, tag="1")]
    pub v: ::prost::alloc::vec::Vec<f64>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ComplexVector {
    #[prost(double, repeated, tag="1")]
    pub a: ::prost::alloc::vec::Vec<f64>,
    #[prost(double, repeated, tag="2")]
    pub b: ::prost::alloc::vec::Vec<f64>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum SweepMode {
    Unspecified = 0,
    Linear = 1,
    Decade = 2,
    Octave = 3,
}
