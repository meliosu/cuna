#![allow(unused)]

#[unsafe(no_mangle)]
pub unsafe extern "C" fn request(id: u32, var: *mut u8) {
    crate::run::request(id, var);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn submit(id: u32, var: *mut u8) {
    crate::run::submit(id, var);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn launch(model: &'static Model) {
    crate::run::launch(model);
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub enum Type {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
    Pointer,
    String,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub enum Device {
    Host,
    Cuda,
}

#[repr(C)]
pub struct Variable {
    pub producers: &'static [u32],
    pub consumers: &'static [u32],
    pub ty: Type,
}

#[repr(C)]
pub struct Operation {
    pub inputs: &'static [u32],
    pub outputs: &'static [u32],
    pub function: unsafe extern "C" fn(),
    pub device: Device,
}

#[repr(C)]
pub struct Model {
    pub variables: &'static [Variable],
    pub operations: &'static [Operation],
    pub inputs: &'static [u32],
    pub outputs: &'static [u32],
}