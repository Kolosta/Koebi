//! Node kinds: static definitions with stable ids, their sockets declared from typed handles.
//!
//! A kind is a marker type implementing [`Kind`], with typed socket handles: `In<K, T>` and
//! `Out<K, T>`, where `K` is the kind and `T` the socket's Rust type. The kind's declarations
//! are built from those same handles ([`In::decl`]), and [`NodeKind::check`], run at compile
//! time, makes sure each declaration sits at its handle's index with an initial value of its
//! type. Reading a node's input through a handle ([`crate::NodeRef::get`]) returns `T`: reading
//! a socket of another kind, or as another type, does not compile.

use core::marker::PhantomData;
use std::collections::BTreeMap;

use koebi_core::LinearRgba;
use koebi_geom::{Affine, Vector};

use crate::value::{EnumKind, Quantity, Socket, SocketType, Value};

/// A node kind's marker type.
pub trait Kind {
    /// Its definition.
    const KIND: &'static NodeKind;
}

/// An input socket of kind `K` holding a `T`.
pub struct In<K, T> {
    index: usize,
    marker: PhantomData<fn() -> (K, T)>,
}

/// An output socket of kind `K` carrying a `T`.
pub struct Out<K, T> {
    index: usize,
    marker: PhantomData<fn() -> (K, T)>,
}

impl<K, T> core::fmt::Debug for In<K, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "In({})", self.index)
    }
}

impl<K, T> core::fmt::Debug for Out<K, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Out({})", self.index)
    }
}

impl<K, T> Clone for In<K, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<K, T> Copy for In<K, T> {}

impl<K, T> Clone for Out<K, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<K, T> Copy for Out<K, T> {}

impl<K, T: Socket> In<K, T> {
    /// The input at `index` of its kind.
    pub const fn new(index: usize) -> Self {
        Self {
            index,
            marker: PhantomData,
        }
    }

    /// Its index among its kind's inputs.
    pub const fn index(self) -> usize {
        self.index
    }

    /// Its declaration, named `name`: its type is `T`'s.
    pub const fn decl(self, name: &'static str) -> InputDecl {
        InputDecl {
            index: self.index,
            name,
            ty: T::TYPE,
            quantity: Quantity::Scalar,
            initial: Initial::None,
            multi: false,
            pass_through: false,
        }
    }
}

impl<K, T: Socket> Out<K, T> {
    /// The output at `index` of its kind.
    pub const fn new(index: usize) -> Self {
        Self {
            index,
            marker: PhantomData,
        }
    }

    /// Its index among its kind's outputs.
    pub const fn index(self) -> usize {
        self.index
    }

    /// Its declaration, named `name`.
    pub const fn decl(self, name: &'static str) -> OutputDecl {
        OutputDecl {
            index: self.index,
            name,
            ty: T::TYPE,
        }
    }
}

/// The value an input holds when a node is made: constant data, turned into a [`Value`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Initial {
    /// No value: an input of data only links carry.
    None,
    /// A boolean.
    Bool(bool),
    /// A whole number.
    Int(i64),
    /// A number.
    Float(f64),
    /// A vector.
    Vector(f64, f64),
    /// A linear color with straight alpha.
    Color([f32; 4]),
    /// The identity transform.
    Identity,
    /// An enumeration's variant.
    Enum(EnumKind, u32),
}

impl Initial {
    /// The value it stands for.
    pub fn value(self) -> Value {
        match self {
            Self::None => Value::None,
            Self::Bool(b) => Value::Bool(b),
            Self::Int(n) => Value::Int(n),
            Self::Float(x) => Value::Float(x),
            Self::Vector(x, y) => Value::Vector(Vector::new(x, y)),
            Self::Color([r, g, b, a]) => Value::Color(LinearRgba::new(r, g, b, a)),
            Self::Identity => Value::Transform(Affine::IDENTITY),
            Self::Enum(kind, index) => Value::Enum(kind, index),
        }
    }

    /// Whether it can be the initial value of an input of type `ty`.
    const fn fits(self, ty: SocketType) -> bool {
        match self {
            Self::None => matches!(
                ty,
                SocketType::Paint
                    | SocketType::Layer
                    | SocketType::Coverage
                    | SocketType::Geometry
                    | SocketType::Text
                    | SocketType::Any
            ),
            Self::Bool(_) => ty.same(SocketType::Bool),
            Self::Int(_) => ty.same(SocketType::Int),
            Self::Float(_) => ty.same(SocketType::Float),
            Self::Vector(..) => ty.same(SocketType::Vector),
            Self::Color(_) => ty.same(SocketType::Color),
            Self::Identity => ty.same(SocketType::Transform),
            Self::Enum(kind, _) => ty.same(SocketType::Enum(kind)),
        }
    }
}

/// An input's declaration.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InputDecl {
    /// Its index among its kind's inputs.
    pub index: usize,
    /// Its name, in English (shown translated).
    pub name: &'static str,
    /// Its type.
    pub ty: SocketType,
    /// What its number means.
    pub quantity: Quantity,
    /// Its value when a node is made.
    pub initial: Initial,
    /// Whether it takes an ordered list of links (a Merge's layers, a Document's pages).
    pub multi: bool,
    /// Whether an output of the same name carries its resolved value.
    pub pass_through: bool,
}

impl InputDecl {
    /// With `quantity`.
    #[must_use]
    pub const fn quantity(mut self, quantity: Quantity) -> Self {
        self.quantity = quantity;
        self
    }

    /// With `initial` as its value when a node is made.
    #[must_use]
    pub const fn initial(mut self, initial: Initial) -> Self {
        self.initial = initial;
        self
    }

    /// Taking an ordered list of links.
    #[must_use]
    pub const fn multi(mut self) -> Self {
        self.multi = true;
        self
    }

    /// With a pass-through output.
    #[must_use]
    pub const fn pass_through(mut self) -> Self {
        self.pass_through = true;
        self
    }
}

/// An output's declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutputDecl {
    /// Its index among its kind's declared outputs.
    pub index: usize,
    /// Its name, in English (shown translated).
    pub name: &'static str,
    /// Its type.
    pub ty: SocketType,
}

/// The families of node kinds, for menus and colors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Family {
    /// The document's output.
    Document,
    /// Pages.
    Page,
    /// Objects: path, group, instance.
    Object,
    /// Geometry: path data, shapes, modifiers.
    Geometry,
    /// Fills and strokes.
    Paint,
    /// Merge, blend, mask, coverage.
    Composition,
    /// Constant values.
    Value,
    /// Preview.
    Inspection,
    /// Node groups and their sockets.
    Group,
}

/// A node kind: a static definition with a stable id.
#[derive(Debug, PartialEq)]
pub struct NodeKind {
    /// Its stable id, as files store it (`koebi.paint.fill`).
    pub id: &'static str,
    /// Its name, in English (shown translated).
    pub name: &'static str,
    /// Its family.
    pub family: Family,
    /// Its inputs, in order.
    pub inputs: &'static [InputDecl],
    /// Its declared outputs, in order; pass-through outputs follow them.
    pub outputs: &'static [OutputDecl],
}

impl NodeKind {
    /// Checks the declarations, at compile time when called in a `const`: each one at its
    /// handle's index, initial values of their input's type, no pass-through on a multi-input.
    ///
    /// ```
    /// use koebi_doc::{Family, In, NodeKind};
    ///
    /// struct Gain;
    /// const AMOUNT: In<Gain, f64> = In::new(0);
    /// const KIND: &NodeKind = &NodeKind {
    ///     id: "example.gain",
    ///     name: "Gain",
    ///     family: Family::Value,
    ///     inputs: &[AMOUNT.decl("Amount")
    ///         .initial(koebi_doc::Initial::Float(1.0))],
    ///     outputs: &[],
    /// };
    /// const _: () = KIND.check();
    /// ```
    ///
    /// A declaration out of its handle's place does not build:
    ///
    /// ```compile_fail
    /// use koebi_doc::{Family, In, NodeKind};
    ///
    /// struct Gain;
    /// const AMOUNT: In<Gain, f64> = In::new(1);
    /// const KIND: &NodeKind = &NodeKind {
    ///     id: "example.gain",
    ///     name: "Gain",
    ///     family: Family::Value,
    ///     inputs: &[AMOUNT.decl("Amount")
    ///         .initial(koebi_doc::Initial::Float(1.0))],
    ///     outputs: &[],
    /// };
    /// const _: () = KIND.check();
    /// ```
    ///
    /// Nor does an initial value of another type:
    ///
    /// ```compile_fail
    /// use koebi_doc::{Family, In, Initial, NodeKind};
    ///
    /// struct Gain;
    /// const AMOUNT: In<Gain, f64> = In::new(0);
    /// const KIND: &NodeKind = &NodeKind {
    ///     id: "example.gain",
    ///     name: "Gain",
    ///     family: Family::Value,
    ///     inputs: &[AMOUNT.decl("Amount").initial(Initial::Bool(true))],
    ///     outputs: &[],
    /// };
    /// const _: () = KIND.check();
    /// ```
    ///
    /// # Panics
    ///
    /// On a declaration out of place, which a `const` turns into a build error.
    pub const fn check(&self) {
        let mut i = 0;
        while i < self.inputs.len() {
            let input = &self.inputs[i];
            assert!(
                input.index == i,
                "an input declared out of its handle's place"
            );
            assert!(
                input.initial.fits(input.ty),
                "an initial value of another type"
            );
            assert!(
                !(input.multi && input.pass_through),
                "a multi-input passed through"
            );
            i += 1;
        }
        let mut o = 0;
        while o < self.outputs.len() {
            assert!(
                self.outputs[o].index == o,
                "an output declared out of its handle's place"
            );
            o += 1;
        }
    }

    /// The type of output `index`: a declared output, or the pass-through of an input.
    pub fn output_type(&self, index: usize) -> Option<SocketType> {
        self.output(index).map(|(_, ty)| ty)
    }

    /// The name and type of output `index`.
    pub fn output(&self, index: usize) -> Option<(&'static str, SocketType)> {
        if let Some(declared) = self.outputs.get(index) {
            return Some((declared.name, declared.ty));
        }
        self.inputs
            .iter()
            .filter(|input| input.pass_through)
            .nth(index - self.outputs.len())
            .map(|input| (input.name, input.ty))
    }

    /// The number of its outputs, pass-through ones included.
    pub fn output_count(&self) -> usize {
        self.outputs.len() + self.inputs.iter().filter(|i| i.pass_through).count()
    }

    /// The pass-through output of input `input`, if it has one.
    pub fn pass_through_of(&self, input: usize) -> Option<usize> {
        let declared = self.inputs.get(input)?;
        declared.pass_through.then(|| {
            self.outputs.len()
                + self.inputs[..input]
                    .iter()
                    .filter(|i| i.pass_through)
                    .count()
        })
    }
}

/// The node kinds a document knows, by id.
#[derive(Clone, Debug, Default)]
pub struct Registry {
    kinds: BTreeMap<&'static str, &'static NodeKind>,
}

impl Registry {
    /// The kinds of the application: the first set (see [`crate::kinds`]).
    pub fn standard() -> Self {
        let mut registry = Self::default();
        for kind in crate::kinds::ALL {
            registry.register(kind);
        }
        registry
    }

    /// Adds `kind`, replacing one of the same id.
    pub fn register(&mut self, kind: &'static NodeKind) {
        self.kinds.insert(kind.id, kind);
    }

    /// The kind with `id`, if known.
    pub fn get(&self, id: &str) -> Option<&'static NodeKind> {
        self.kinds.get(id).copied()
    }

    /// Every kind, by id.
    pub fn kinds(&self) -> impl Iterator<Item = &'static NodeKind> + '_ {
        self.kinds.values().copied()
    }
}
