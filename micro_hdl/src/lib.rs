use crate::context::Context;
use crate::signal::Signal;

pub mod backend;
pub mod context;
pub mod primitive;
pub mod signal;

pub trait Module: ModuleInstance + std::any::Any {}

pub enum PinType {
    Input,
    Output,
    InOut,
}

pub struct ModulePin {
    pub name: String,
    pub pin_type: PinType,
}

pub struct InstancePin {
    pub signal: Signal,
}

pub trait ModuleInstance {
    fn generate(&self, c: &mut Context) -> Vec<InstancePin>;
    fn spice(&self) -> String;
    fn name(&self) -> String;
    fn get_module_pins(&self) -> Vec<ModulePin>;
    fn get_instance_pins(&self) -> Vec<InstancePin>;
    fn config(&self) -> ModuleConfig;
}

#[derive(Debug, Eq, PartialEq)]
pub enum ModuleConfig {
    Raw,
    Generate,
}
