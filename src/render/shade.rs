// Copyright 2014 The Gfx-rs Developers.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Shader parameter handling.

use std::cell::Cell;
use std::rc::Rc;
use device::shade as s;
use device::{RawBufferHandle, ProgramHandle, SamplerHandle, TextureHandle};

/// Helper trait to transform base types into their corresponding uniforms
pub trait ToUniform {
    /// Create a `UniformValue` representing this value.
    fn to_uniform(&self) -> s::UniformValue;
}

macro_rules! impl_ToUniform(
    ($srcty:ty, $dstty:expr) => (
        impl ToUniform for $srcty {
            fn to_uniform(&self) -> s::UniformValue {
                $dstty(*self)
            }
        }
    );
)

impl_ToUniform!(i32, s::ValueI32)
impl_ToUniform!(f32, s::ValueF32)

impl_ToUniform!([i32, ..2], s::ValueI32Vector2)
impl_ToUniform!([i32, ..3], s::ValueI32Vector3)
impl_ToUniform!([i32, ..4], s::ValueI32Vector4)

impl_ToUniform!([f32, ..2], s::ValueF32Vector2)
impl_ToUniform!([f32, ..3], s::ValueF32Vector3)
impl_ToUniform!([f32, ..4], s::ValueF32Vector4)

impl_ToUniform!([[f32, ..2], ..2], s::ValueF32Matrix2)
impl_ToUniform!([[f32, ..3], ..3], s::ValueF32Matrix3)
impl_ToUniform!([[f32, ..4], ..4], s::ValueF32Matrix4)

/// Variable index of a uniform.
pub type VarUniform = u16;

/// Variable index of a uniform block.
pub type VarBlock = u8;

/// Variable index of a texture.
pub type VarTexture = u8;

/// A texture parameter: consists of a texture handle with an optional sampler.
pub type TextureParam = (TextureHandle, Option<SamplerHandle>);

/// A borrowed mutable storage for shader parameter values.
// Not sure if it's the best data structure to represent it.
pub struct ParamValues<'a> {
    /// uniform values to be provided
    pub uniforms: &'a mut [Option<s::UniformValue>],
    /// uniform buffers to be provided
    pub blocks  : &'a mut [Option<RawBufferHandle>],
    /// textures to be provided
    pub textures: &'a mut [Option<TextureParam>],
}

/// Encloses a shader program handle with its parameter
pub trait Program {
    /// Get the contained program handle
    fn get_handle(&self) -> &ProgramHandle;
    /// Get all the contained parameter values
    fn fill_params(&self, ParamValues);
}

/// An error type on either the parameter storage or the program side
#[deriving(Clone, PartialEq, Show)]
pub enum ParameterError {
    /// Internal error
    ErrorInternal,
    /// Error with the named uniform
    ErrorUniform(String),
    /// Error with the named uniform block
    ErrorBlock(String),
    /// Error with the named texture.
    ErrorTexture(String),
}

/// Abstracts the shader parameter structure, generated by the `shader_param` attribute
pub trait ShaderParam<L> {
    /// Creates a new link, self is passed as a workaround for Rust to not be lost in generics
    fn create_link(Option<Self>, &s::ProgramInfo) -> Result<L, ParameterError>;
    /// Get all the contained parameter values, using a given link.
    fn fill_params(&self, &L, ParamValues);
}

impl ShaderParam<()> for () {
    fn create_link(_: Option<()>, info: &s::ProgramInfo) -> Result<(), ParameterError> {
        match info.uniforms.as_slice().head() {
            Some(u) => return Err(ErrorUniform(u.name.clone())),
            None => (),
        }
        match info.blocks.as_slice().head() {
            Some(b) => return Err(ErrorBlock(b.name.clone())),
            None => (),
        }
        match info.textures.as_slice().head() {
            Some(t) => return Err(ErrorTexture(t.name.clone())),
            None => (),
        }
        Ok(())
    }

    fn fill_params(&self, _: &(), _: ParamValues) {
        //empty
    }
}

/// A bundle that encapsulates a program and a custom user-provided
/// structure containing the program parameters.
/// # Type parameters:
///
/// * `L` - auto-generated structure that has a variable index for every field of T
/// * `T` - user-provided structure containing actual parameter values
pub struct UserProgram<L, T> {
    /// Shader program handle
    program: ProgramHandle,
    /// Hidden link that provides parameter indices for user data
    link: L,
}

impl<L: Copy, T> Clone for UserProgram<L, T> {
    fn clone(&self) -> UserProgram<L, T> {
        UserProgram {
            program: self.program.clone(),
            link: self.link,
        }
    }
}

impl<L, T: ShaderParam<L>> UserProgram<L, T> {
    /// Connect a shader program with a parameter structure
    pub fn connect(prog: ProgramHandle) ->
                   Result<UserProgram<L, T>, ParameterError> {
        ShaderParam::create_link(None::<T>, prog.get_info())
            .map(|link| UserProgram {
                program: prog.clone(),
                link: link,
        })
    }
}

// Tuple of references `(&MyProgram, &Data)` is the standard way of providing
// a program with its parameters for the draw call.

impl<'a, L, T: ShaderParam<L>> Program for (&'a UserProgram<L, T>, &'a T) {
    fn get_handle(&self) -> &ProgramHandle {
        &self.val0().program
    }

    fn fill_params(&self, params: ParamValues) {
        self.val1().fill_params(&self.val0().link, params);
    }
}


pub type EmptyProgram = UserProgram<(), ()>;

impl<'a> Program for &'a EmptyProgram {
    fn get_handle(&self) -> &ProgramHandle {
        &self.program
    }

    fn fill_params(&self, params: ParamValues) {
        debug_assert!(
            params.uniforms.is_empty() &&
            params.blocks.is_empty() &&
            params.textures.is_empty(),
            "trying to bind a program handle that has uniforms;\n
            please link with `DeviceHelper::link_program` instead,\n
            or connect with `UserProgram::connect` after construction"
        );
    }
}


/// A named cell containing arbitrary value
pub struct NamedCell<T> {
    /// Name
    pub name: String,
    /// Value
    pub value: Cell<T>,
}

/// A dictionary of parameters, meant to be shared between different programs
pub struct ParamDictionary {
    /// Uniform dictionary
    pub uniforms: Vec<NamedCell<s::UniformValue>>,
    /// Block dictionary
    pub blocks: Vec<NamedCell<RawBufferHandle>>,
    /// Texture dictionary
    pub textures: Vec<NamedCell<TextureParam>>,
}

/// An associated link structure for `ParamDictionary` that redirects program
/// input to the relevant dictionary cell.
pub struct ParamDictionaryLink {
    uniforms: Vec<uint>,
    blocks: Vec<uint>,
    textures: Vec<uint>,
}

/// A shader program with dictionary of parameters
pub struct DictionaryProgram {
    program: ProgramHandle,
    link: ParamDictionaryLink,
    data: Rc<ParamDictionary>,
}

impl DictionaryProgram {
    /// Connect a shader program with a parameter structure
    pub fn connect(prog: ProgramHandle, data: Rc<ParamDictionary>)
                   -> Result<DictionaryProgram, ParameterError> {
        //TODO: proper error checks
        let link = ParamDictionaryLink {
            uniforms: prog.get_info().uniforms.iter().map(|var|
                data.uniforms.iter().position(|c| c.name == var.name).unwrap()
            ).collect(),
            blocks: prog.get_info().blocks.iter().map(|var|
                data.blocks  .iter().position(|c| c.name == var.name).unwrap()
            ).collect(),
            textures: prog.get_info().textures.iter().map(|var|
                data.textures.iter().position(|c| c.name == var.name).unwrap()
            ).collect(),
        };
        Ok(DictionaryProgram {
            program: prog,
            link: link,
            data: data,
        })
    }
}

impl<'a> Program for &'a DictionaryProgram {
    fn get_handle(&self) -> &ProgramHandle {
        &self.program
    }

    fn fill_params(&self, params: ParamValues) {
        for (&id, var) in self.link.uniforms.iter().zip(params.uniforms.mut_iter()) {
            *var = Some(self.data.uniforms[id].value.get());
        }
        for (&id, var) in self.link.blocks.iter().zip(params.blocks.mut_iter()) {
            *var = Some(self.data.blocks[id].value.get());
        }
        for (&id, var) in self.link.textures.iter().zip(params.textures.mut_iter()) {
            *var = Some(self.data.textures[id].value.get());
        }
    }
}
